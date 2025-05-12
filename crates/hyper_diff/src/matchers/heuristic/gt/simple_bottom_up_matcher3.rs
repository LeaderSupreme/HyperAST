use super::bottom_up_matcher::BottomUpMatcher;
use crate::decompressed_tree_store::{
    ContiguousDescendants, DecompressedTreeStore, DecompressedWithParent, POBorrowSlice, PostOrder,
    PostOrderIterable, PostOrderKeyRoots,
};
use crate::decompressed_tree_store::{ShallowDecompressedTreeStore, SimpleZsTree as ZsTree};
use crate::matchers::mapping_store::MonoMappingStore;
use crate::matchers::similarity_metrics;
use hyperast::types::{
    DecompressedFrom, HyperAST, NodeId, NodeStore, Tree, TypeStore, WithChildren, WithHashs,
};
use hyperast::PrimInt;
use std::fmt::Debug;
use std::{collections::HashMap, hash::Hash};
pub struct SimpleBottomUpMatcher<
    Dsrc,
    Ddst,
    HAST,
    M: MonoMappingStore,
    const SIMILARITY_THRESHOLD_NUM: u64 = 1,
    const SIMILARITY_THRESHOLD_DEN: u64 = 2,
> {
    internal: BottomUpMatcher<Dsrc, Ddst, HAST, M>,
}

impl<
        'a,
        Dsrc: DecompressedTreeStore<HAST, M::Src>
            + DecompressedWithParent<HAST, M::Src>
            + PostOrder<HAST, M::Src>
            + PostOrderIterable<HAST, M::Src>
            + DecompressedFrom<HAST, Out = Dsrc>
            + ContiguousDescendants<HAST, M::Src>
            + POBorrowSlice<HAST, M::Src>,
        Ddst: DecompressedTreeStore<HAST, M::Dst>
            + DecompressedWithParent<HAST, M::Dst>
            + PostOrder<HAST, M::Dst>
            + PostOrderIterable<HAST, M::Dst>
            + DecompressedFrom<HAST, Out = Ddst>
            + ContiguousDescendants<HAST, M::Dst>
            + POBorrowSlice<HAST, M::Dst>,
        HAST: HyperAST + Copy,
        M: MonoMappingStore + Default,
        const SIMILARITY_THRESHOLD_NUM: u64,
        const SIMILARITY_THRESHOLD_DEN: u64,
    > SimpleBottomUpMatcher<Dsrc, Ddst, HAST, M, SIMILARITY_THRESHOLD_NUM, SIMILARITY_THRESHOLD_DEN>
where
    for<'t> <HAST as hyperast::types::AstLending<'t>>::RT: WithHashs,
    M::Src: PrimInt,
    M::Dst: PrimInt,
    HAST::Label: Eq,
    HAST::IdN: Debug,
    HAST::IdN: NodeId<IdN = HAST::IdN>,
{
    pub fn new(stores: HAST, src_arena: Dsrc, dst_arena: Ddst, mappings: M) -> Self {
        Self {
            internal: BottomUpMatcher {
                stores,
                src_arena,
                dst_arena,
                mappings,
            },
        }
    }

    pub fn match_it(
        mapping: crate::matchers::Mapper<HAST, Dsrc, Ddst, M>,
    ) -> crate::matchers::Mapper<HAST, Dsrc, Ddst, M> {
        let mut matcher = Self {
            internal: BottomUpMatcher {
                stores: mapping.hyperast,
                src_arena: mapping.mapping.src_arena,
                dst_arena: mapping.mapping.dst_arena,
                mappings: mapping.mapping.mappings,
            },
        };
        matcher.internal.mappings.topit(
            matcher.internal.src_arena.len(),
            matcher.internal.dst_arena.len(),
        );
        Self::execute(&mut matcher);
        crate::matchers::Mapper {
            hyperast: mapping.hyperast,
            mapping: crate::matchers::Mapping {
                src_arena: matcher.internal.src_arena,
                dst_arena: matcher.internal.dst_arena,
                mappings: matcher.internal.mappings,
            },
        }
    }

    pub fn matchh(store: HAST, src: &'a HAST::IdN, dst: &'a HAST::IdN, mappings: M) -> Self {
        let mut matcher = Self::new(
            store,
            Dsrc::decompress(store, src),
            Ddst::decompress(store, dst),
            mappings,
        );
        matcher.internal.mappings.topit(
            matcher.internal.src_arena.len(),
            matcher.internal.dst_arena.len(),
        );
        Self::execute(&mut matcher);
        matcher
    }

    pub fn execute<'b>(&mut self) {
        assert!(self.internal.src_arena.len() > 0);
        let similarity_threshold: f64 =
            SIMILARITY_THRESHOLD_NUM as f64 / SIMILARITY_THRESHOLD_DEN as f64;

        for tree in self.internal.src_arena.iter_df_post::<true>() {
            if self.internal.src_arena.parent(&tree).is_none() {
                self.internal.mappings.link(
                    self.internal.src_arena.root(), // <- this is tree
                    self.internal.dst_arena.root(),
                );
                self.last_chance_match(
                    &self.internal.src_arena.root(), // <- this is tree
                    &self.internal.dst_arena.root(),
                );
                break;
            } else if !(self.internal.mappings.is_src(&tree) || !self.src_has_children(tree)) {
                let candidates = self.internal.get_dst_candidates(&tree);
                let mut best = None;
                let mut max: f64 = -1.;

                // Can be used to calculate an appropriate threshold. In Gumtree this is done when no threshold is provided.
                // let tree_size = self.internal.src_arena.descendants_count(&tree);

                for candidate in candidates {
                    // In gumtree implementation they check if Simliarity_THreshold is set, otherwise they compute a fitting value
                    // !TODO -> should &[tree] be self.internal.src_arena.descendants(&tree)??
                    let similarity = similarity_metrics::chawathe_similarity(
                        &[tree],
                        &[candidate],
                        &self.internal.mappings,
                    );

                    if (similarity > max && similarity >= similarity_threshold) {
                        max = similarity;
                        best = Some(candidate);
                    }
                }

                if let Some(best_candidate) = best {
                    self.last_chance_match(&tree, &best_candidate);
                    self.internal.mappings.link(tree, best_candidate);
                }
            } else if self.internal.mappings.is_src(&tree)  // Check if there are unmapped children in src or dst
                && self.internal.are_srcs_unmapped(&tree)
                && self
                    .internal
                    .are_dsts_unmapped(&self.internal.mappings.get_dst_unchecked(&tree))
            {
                self.last_chance_match(&tree, &self.internal.mappings.get_dst_unchecked(&tree));
            }
        }
    }

    fn src_has_children(&mut self, src: M::Src) -> bool {
        use num_traits::ToPrimitive;
        let r = self
            .internal
            .stores
            .node_store()
            .resolve(&self.internal.src_arena.original(&src))
            .has_children();

        assert_eq!(
            r,
            self.internal.src_arena.lld(&src) < src,
            "{:?} {:?}",
            self.internal.src_arena.lld(&src),
            src.to_usize()
        );
        r
    }

    fn last_chance_match(&mut self, src: &M::Src, dst: &M::Dst) {
        self.internal.lcs_equal_matching(src, dst);
        self.internal.lcs_structure_matching(src, dst);
        self.histogram_matching(src, dst);
    }

    // Almost exactly the same as the histogram matching from self.internal, but here the early escapes with checking if unmapped are added
    fn histogram_matching(&mut self, src: &M::Src, dst: &M::Dst) {
        let mut src_histogram: HashMap<<HAST::TS as TypeStore>::Ty, Vec<M::Src>> = HashMap::new(); //Map<Type, List<ITree>>
        for c in self.internal.src_arena.children(src) {
            if self.internal.are_srcs_unmapped(&c) {
                let t = &self
                    .internal
                    .stores
                    .resolve_type(&self.internal.src_arena.original(&c));
                if !src_histogram.contains_key(t) {
                    src_histogram.insert(*t, vec![]);
                }
                src_histogram.get_mut(t).unwrap().push(c);
            }
        }

        let mut dst_histogram: HashMap<<HAST::TS as TypeStore>::Ty, Vec<M::Dst>> = HashMap::new(); //Map<Type, List<ITree>>
        for c in self.internal.dst_arena.children(dst) {
            if self.internal.are_dsts_unmapped(&c) {
                let t = &self
                    .internal
                    .stores
                    .resolve_type(&self.internal.dst_arena.original(&c));
                if !dst_histogram.contains_key(t) {
                    dst_histogram.insert(*t, vec![]);
                }
                dst_histogram.get_mut(t).unwrap().push(c);
            }
        }
        for t in src_histogram.keys() {
            if dst_histogram.contains_key(t)
                && src_histogram[t].len() == 1
                && dst_histogram[t].len() == 1
            {
                let t1 = src_histogram[t][0];
                let t2 = dst_histogram[t][0];
                if self.internal.mappings.link_if_both_unmapped(t1, t2) {
                    self.internal.last_chance_match_histogram(&t1, &t2);
                }
            }
        }
    }
}

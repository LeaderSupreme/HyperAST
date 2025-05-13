use crate::actions::Actions;
use crate::algorithms;
use crate::{actions::action_vec, algorithms::gumtree::diff};
use hyperast::types::NodeId;
use hyperast::{
    full::FullNode, nodes::SyntaxSerializer, store::SimpleStores, tree_gen::StatsGlobalData,
};
use hyperast_gen_ts_java::{
    legion_with_refs::{self, JavaTreeGen, Local},
    types::TStore,
};

//Parses the provided bytes to a java syntax tree
fn preprocess_for_diff(
    src: &[u8],
    dst: &[u8],
) -> (
    SimpleStores<TStore>,
    FullNode<StatsGlobalData, Local>,
    FullNode<StatsGlobalData, Local>,
) {
    let mut stores = SimpleStores::<TStore>::default();
    let mut md_cache = Default::default(); // [cite: 133, 139]
    let mut java_tree_gen = JavaTreeGen::new(&mut stores, &mut md_cache);
    let tree = match legion_with_refs::tree_sitter_parse(src) {
        Ok(t) => t,
        Err(t) => t,
    };
    let src = java_tree_gen.generate_file(b"", src, tree.walk());
    let tree = match legion_with_refs::tree_sitter_parse(dst) {
        Ok(t) => t,
        Err(t) => t,
    };
    let dst = java_tree_gen.generate_file(b"", dst, tree.walk());
    return (stores, src, dst);
}

fn prepare_tree_print<'a>(
    stores: &'a SimpleStores<TStore>,
) -> impl Fn(&FullNode<StatsGlobalData, Local>) -> () + 'a {
    return |tree: &FullNode<StatsGlobalData, Local>| {
        println!();
        println!(
            "{}",
            SyntaxSerializer::new(stores, tree.local.compressed_node)
        );
    };
}

#[test]
fn change_method_name_test() {
    let src = "class A {}".as_bytes();
    let dst = "class B {}".as_bytes();

    let (stores, src, dst) = preprocess_for_diff(src, dst);

    let diff_result = algorithms::gumtree::diff_simple(
        &stores,
        &src.local.compressed_node,
        &dst.local.compressed_node,
    );

    let print_tree = prepare_tree_print(&stores);
    print_tree(&src);
    print_tree(&dst);

    println!("stats from diffing: \n{}", &diff_result.summarize());
    // diff_result
    //     .actions
    //     .unwrap()
    //     .iter()
    //     .for_each(|a| println!("{:?}", a));

    action_vec::actions_vec_f(
        &diff_result.actions.as_ref().unwrap(),
        &diff_result.mapper.hyperast,
        src.local.compressed_node.as_id().clone(),
    );

    // There should be only one action, to update the method name.
    assert_eq!(diff_result.actions.unwrap().len(), 1 as usize);
}

#[test]
fn example_paper_test() {
    let src = "public class Foo {public void foo() {print('unchanged'); print('unchanged'); print('original');}}".as_bytes();
    let dst = "public class Foo {public void foo() {print('unchanged'); print('unchanged'); print('modified');}}".as_bytes();

    let (stores, src, dst) = preprocess_for_diff(src, dst);
    let diff_result = algorithms::gumtree::diff(
        &stores,
        &src.local.compressed_node,
        &dst.local.compressed_node,
    );

    let print_tree = prepare_tree_print(&stores);
    print_tree(&src);
    print_tree(&dst);

    println!("stats from diffing: \n{}", &diff_result.summarize());

    action_vec::actions_vec_f(
        &diff_result.actions.as_ref().unwrap(),
        &diff_result.mapper.hyperast,
        src.local.compressed_node.as_id().clone(),
    );

    // There should only be one action, update 'original' to 'modified':
    // Upd "'modified'" (83, 93, Entity(576))
    assert_eq!(diff_result.actions.unwrap().len(), 1 as usize);
}

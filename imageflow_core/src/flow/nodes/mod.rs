
use daggy::{Dag, EdgeIndex, NodeIndex};
use ffi::{Context, Job, PixelFormat, EdgeKind, BitmapBgra};
use libc::{int32_t, size_t};
use petgraph::EdgeDirection;
mod rotate_flip_transpose;
mod clone_crop_fill_expand;
mod scale_render;
mod create_canvas;
mod codecs_and_pointer;

extern crate imageflow_serde as s;
pub use self::clone_crop_fill_expand::CLONE;
pub use self::clone_crop_fill_expand::COPY_RECT;
pub use self::clone_crop_fill_expand::CROP;
pub use self::clone_crop_fill_expand::CROP_MUTATE;
pub use self::clone_crop_fill_expand::EXPAND_CANVAS;
pub use self::clone_crop_fill_expand::FILL_RECT;
pub use self::codecs_and_pointer::BITMAP_BGRA_POINTER;
pub use self::codecs_and_pointer::DECODER;
pub use self::codecs_and_pointer::ENCODE;

// TODO: Implement decoder IDCT


pub use self::codecs_and_pointer::PRIMITIVE_DECODER;
pub use self::create_canvas::CREATE_CANVAS;
pub use self::rotate_flip_transpose::APPLY_ORIENTATION;
pub use self::rotate_flip_transpose::FLIP_H;
pub use self::rotate_flip_transpose::FLIP_H_PRIMITIVE;
pub use self::rotate_flip_transpose::FLIP_V;
pub use self::rotate_flip_transpose::FLIP_V_PRIMITIVE;
pub use self::rotate_flip_transpose::NO_OP;
pub use self::rotate_flip_transpose::ROTATE_180;
pub use self::rotate_flip_transpose::ROTATE_270;
pub use self::rotate_flip_transpose::ROTATE_90;
pub use self::rotate_flip_transpose::TRANSPOSE;
pub use self::scale_render::SCALE;
pub use self::scale_render::SCALE_1D;
pub use self::scale_render::SCALE_1D_TO_CANVAS_1D;
use super::definitions::*;

struct NodeDefHelpers {}
impl NodeDefHelpers {
    fn copy_frame_est_from_first_input(ctx: &mut OpCtxMut, ix: NodeIndex<u32>) {
        ctx.copy_frame_est_from_first_input(ix);
    }
    fn copy_frame_est_from_first_canvas(ctx: &mut OpCtxMut, ix: NodeIndex<u32>) {
        ctx.copy_frame_est_from_first_canvas(ix);
    }
    fn rotate_frame_info(ctx: &mut OpCtxMut, ix: NodeIndex<u32>) {
        ctx.rotate_frame_est_from_first_input(ix);
    }
    fn flatten_flip_v(ctx: &mut OpCtxMut, ix: NodeIndex<u32>) {
        // ctx.graph.node_weight_mut(ix).unwrap()
    }

    fn delete_node_and_snap_together(ctx: &mut OpCtxMut, ix: NodeIndex<u32>) {
        ctx.delete_node_and_snap_together(ix);
    }
}

impl<'c> OpCtxMut<'c> {
    pub fn first_parent_of_kind<'a>(&'a self,
                                    of_node: NodeIndex<u32>,
                                    filter_by_kind: EdgeKind)
                                    -> Option<NodeIndex<u32>> {
        self.graph
            .graph()
            .edges_directed(of_node, EdgeDirection::Incoming)
            .filter(|&(node, &kind)| kind == filter_by_kind)
            .map(|(node, kind)| node)
            .nth(0)
    }

    pub fn first_parent_input<'a>(&'a self, of_node: NodeIndex<u32>) -> Option<NodeIndex<u32>> {
        self.first_parent_of_kind(of_node, EdgeKind::Input)
    }
    pub fn first_parent_canvas<'a>(&'a self, of_node: NodeIndex<u32>) -> Option<NodeIndex<u32>> {
        self.first_parent_of_kind(of_node, EdgeKind::Canvas)
    }

    pub fn first_parent_input_weight<'a>(&'a self, of_node: NodeIndex<u32>) -> Option<Node> {
        self.first_parent_input(of_node).map(|ix| self.graph.node_weight(ix).unwrap().clone())
    }


    pub fn first_parent_frame_info_some<'a>(&'a self,
                                            of_node: NodeIndex<u32>)
                                            -> Option<FrameInfo> {
        self.first_parent_input(of_node).and_then(|ix| {
            self.graph.node_weight(ix).and_then(|w| {
                match w.frame_est {
                    FrameEstimate::Some(ref frame_info) => Some(*frame_info),
                    _ => None,
                }
            })
        })
    }

    pub fn get_json_params<'a>(&'a self, ix: NodeIndex<u32>) -> Option<s::Node> {
        self.graph.node_weight(ix).and_then(|w| {
            match w.params {
                NodeParams::Json(ref node) => Some(node.clone()),
                _ => None,
            }
        })
    }

    pub fn first_parent_canvas_weight<'a>(&'a self, of_node: NodeIndex<u32>) -> Option<&Node> {
        self.first_parent_canvas(of_node).map(|ix| self.graph.node_weight(ix).unwrap())
    }

    pub fn first_parent_result_frame<'a, 'b>(&'a self,
                                             of_node: NodeIndex<u32>,
                                             kind: EdgeKind)
                                             -> Option<*mut BitmapBgra> {
        self.first_parent_of_kind(of_node, kind)
            .and_then(|ix| self.graph.node_weight(ix))
            .and_then(|w| match w.result {
                NodeResult::Frame(ptr) => Some(ptr),
                _ => None,
            })
    }



    pub fn first_parent_input_weight_mut<'a>(&'a mut self,
                                             of_node: NodeIndex<u32>)
                                             -> Option<&mut Node> {
        self.first_parent_input(of_node).map(move |ix| self.graph.node_weight_mut(ix).unwrap())
    }

    pub fn has_other_children<'a>(&'a self,
                                  of_node: NodeIndex<u32>,
                                  except_child: NodeIndex<u32>)
                                  -> bool {
        self.graph
            .graph()
            .neighbors_directed(of_node, EdgeDirection::Outgoing)
            .any(|n| n != except_child)
    }

    pub fn weight<'a>(&'a self, ix: NodeIndex<u32>) -> &'a Node {
        self.graph.node_weight(ix).unwrap()
    }

    pub fn weight_mut<'a>(&'a mut self, node_to_update: NodeIndex<u32>) -> &'a mut Node {
        self.graph.node_weight_mut(node_to_update).unwrap()
    }
    pub fn copy_frame_est_from_first_input<'a>(&'a mut self, node_to_update: NodeIndex<u32>) {
        match self.first_parent_input(node_to_update) {
            Some(input_ix) => {
                if self.graph.node_weight(input_ix).unwrap().frame_est == FrameEstimate::None {
                    panic!("Parent frame {} is not estimated", input_ix.index());
                }
                self.graph.node_weight_mut(node_to_update).unwrap().frame_est =
                    self.graph.node_weight(input_ix).unwrap().frame_est.clone();
            }
            None => {}
        }
    }
    pub fn copy_frame_est_from_first_canvas<'a>(&'a mut self, node_to_update: NodeIndex<u32>) {
        match self.first_parent_canvas(node_to_update) {
            Some(input_ix) => {
                self.graph.node_weight_mut(node_to_update).unwrap().frame_est =
                    self.graph.node_weight(input_ix).unwrap().frame_est.clone();
            }
            None => {}
        }
    }

    pub fn assert_ok(&self) {
        unsafe {
            ::ContextPtr::from_ptr(self.c).assert_ok(None);
        }
    }

    pub fn rotate_frame_est_from_first_input<'a, 'b>(&'a mut self,
                                                     node_to_update: NodeIndex<u32>) {
        // TODO: select by EdgeKind=Input
        let input = self.graph
            .graph()
            .neighbors_directed(node_to_update, EdgeDirection::Incoming)
            .nth(0);
        match input {
            Some(input_ix) => {
                let input_est = self.graph.node_weight(input_ix).unwrap().frame_est.clone();
                let mut w = self.graph.node_weight_mut(node_to_update).unwrap();
                w.frame_est = match input_est {
                    FrameEstimate::Some(info) => {
                        FrameEstimate::Some(FrameInfo {
                            w: info.h,
                            h: info.w,
                            ..info
                        })
                    }
                    FrameEstimate::UpperBound(info) => {
                        FrameEstimate::UpperBound(FrameInfo {
                            w: info.h,
                            h: info.w,
                            ..info
                        })
                    }
                    other => other,
                };
            }
            None => {}
        }
    }

    pub fn copy_edges_to<'a>(&'a mut self,
                             from_node: NodeIndex<u32>,
                             to_node: NodeIndex<u32>,
                             direction: EdgeDirection) {
        let edges = self.graph
            .graph()
            .edges_directed(from_node, direction)
            .map(|(a, b)| (a, b.clone()))
            .collect::<Vec<_>>();

        for (other_node, weight) in edges {
            match direction {
                EdgeDirection::Incoming => {
                    self.graph.add_edge(other_node, to_node, weight.clone()).unwrap()
                }
                EdgeDirection::Outgoing => {
                    self.graph.add_edge(to_node, other_node, weight.clone()).unwrap()
                }
            };
        }
    }
    pub fn delete_child_edges_for<'a>(&'a mut self, from_node: NodeIndex<u32>) {
        loop {
            match self.graph
                .raw_edges()
                .iter()
                .position(|e| e.source() == from_node)
                .and_then(|ix| self.graph.remove_edge(EdgeIndex::new(ix))) {
                None => {
                    break;
                }
                _ => {}
            }
        }
    }

    pub fn delete_node_and_snap_together<'a>(&'a mut self, node_to_delete: NodeIndex<u32>) {
        // Prefer EdgeKind=Input
        let input = self.graph
            .graph()
            .neighbors_directed(node_to_delete, EdgeDirection::Incoming)
            .nth(0);
        match input {
            None => {}
            Some(from_node) => {
                self.copy_edges_to(node_to_delete, from_node, EdgeDirection::Outgoing);
                self.graph.remove_node(node_to_delete).unwrap();
            }
        };
    }

    // Links nodes with Input edges
    pub fn replace_node<'a>(&'a mut self, index: NodeIndex<u32>, with_list: Vec<Node>) {
        let mut with = with_list.clone();
        match with.len() {
            0 => self.delete_node_and_snap_together(index),
            n => {
                with.reverse();
                let mut last_ix = self.graph.add_node(with.pop().unwrap());
                self.copy_edges_to(index, last_ix, EdgeDirection::Incoming);
                while with.len() > 0 {
                    let new_ix = self.graph.add_node(with.pop().unwrap());
                    self.graph.add_edge(last_ix, new_ix, EdgeKind::Input).unwrap();
                    last_ix = new_ix;
                }
                self.copy_edges_to(index, last_ix, EdgeDirection::Outgoing);
                self.graph.remove_node(index).unwrap();
            }
        }
    }

    pub fn replace_node_with_existing<'a>(&'a mut self,
                                          index: NodeIndex<u32>,
                                          with_index: NodeIndex<u32>) {
        self.copy_edges_to(index, with_index, EdgeDirection::Incoming);
        self.copy_edges_to(index, with_index, EdgeDirection::Outgoing);
        self.graph.remove_node(index).unwrap();
    }
}
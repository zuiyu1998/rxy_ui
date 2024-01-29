use crate::{EntityWorldRef, Result, StyleSheetsInfo};
use alloc::collections::BinaryHeap;
use bevy_ecs::prelude::Entity;
use bevy_utils::HashMap;
use rxy_bevy::BevyRenderer;
use rxy_core::prelude::{Either, EitherExt};
use rxy_style::{
    NodeAttrStyleItemId, NodeStyleAttrInfo, NodeStyleItemId, NodeStyleSheetId,
    StyleAttrId, StyleError, StyleSheetId, StyleSheetIndex, StyleSheetLocation,
};

use crate::{AppliedStyleSheet, ApplyStyleSheetsMemberState, AttrStyleOwner, StyleSheetDefinition};

#[derive(Default, Clone, Debug)]
pub struct NodeStyleSheetsState {
    pub inline_style_sheet: Vec<Option<StyleSheetDefinition>>,
    // pub shared_style_sheet_ids: Vec<Option<SharedStyleSheetId>>,
    pub shared_style_sheet_ids: Vec<Option<StyleSheetId<BevyRenderer>>>,
}

impl FromIterator<AppliedStyleSheet> for NodeStyleSheetsState {
    fn from_iter<T: IntoIterator<Item = AppliedStyleSheet>>(iter: T) -> Self {
        let mut r = NodeStyleSheetsState::default();
        for item in iter.into_iter() {
            match item {
                AppliedStyleSheet::None => {}
                AppliedStyleSheet::Inline(style_sheet) => {
                    r.inline_style_sheet.push(Some(style_sheet));
                }
                AppliedStyleSheet::Shared(style_sheet_id) => {
                    r.shared_style_sheet_ids.push(Some(style_sheet_id));
                }
            }
        }
        r
    }
}

impl NodeStyleSheetsState {
    pub fn apply_as_shared(
        &self,
        entity: Entity,
        index: StyleSheetIndex,
    ) -> impl Iterator<Item = AppliedStyleSheet> + Send + 'static {
        let shared_style_sheet_ids = self.shared_style_sheet_ids.clone();
        let inline_style_sheet_len = self.inline_style_sheet.len();
        (0..inline_style_sheet_len)
            .map(move |i| {
                AppliedStyleSheet::Shared(StyleSheetId {
                    node_style_sheet_id: NodeStyleSheetId {
                        index: index + i as StyleSheetIndex,
                        location: StyleSheetLocation::Inline,
                    },
                    node_id: entity,
                })
            })
            .chain(
                shared_style_sheet_ids
                    .into_iter()
                    .flatten()
                    .map(|n| AppliedStyleSheet::Shared(n.clone())),
            )
    }

    pub fn style_sheets_info(&self) -> StyleSheetsInfo {
        StyleSheetsInfo {
            inline_style_sheet_count: self.inline_style_sheet.len() as _,
            shared_style_sheet_count: self.shared_style_sheet_ids.len() as _,
        }
    }
}

impl NodeStyleSheetsState {
    pub fn get_inline_style_sheet(
        &self,
        style_sheet_index: StyleSheetIndex,
    ) -> Result<&StyleSheetDefinition> {
        self.inline_style_sheet
            .get(style_sheet_index as usize)
            .ok_or(StyleError::NoFoundStyleSheetOnNode(NodeStyleSheetId {
                location: StyleSheetLocation::Inline,
                index: style_sheet_index,
            }))?
            .as_ref()
            .ok_or(StyleError::RemovedStyleSheet(NodeStyleSheetId {
                location: StyleSheetLocation::Inline,
                index: style_sheet_index,
            }))
    }
    pub fn get_inline_style_sheet_mut(
        &mut self,
        style_sheet_index: StyleSheetIndex,
    ) -> Result<&mut StyleSheetDefinition> {
        self.inline_style_sheet
            .get_mut(style_sheet_index as usize)
            .ok_or(StyleError::NoFoundStyleSheetOnNode(NodeStyleSheetId {
                location: StyleSheetLocation::Inline,
                index: style_sheet_index,
            }))?
            .as_mut()
            .ok_or(StyleError::RemovedStyleSheet(NodeStyleSheetId {
                location: StyleSheetLocation::Inline,
                index: style_sheet_index,
            }))
    }

    pub fn get_shared_style_sheet_id(
        &self,
        style_sheet_index: StyleSheetIndex,
    ) -> Result<StyleSheetId<BevyRenderer>> {
        let style_sheet_id = self
            .shared_style_sheet_ids
            .get(style_sheet_index as usize)
            .ok_or(StyleError::NoFoundStyleSheetOnNode(NodeStyleSheetId {
                location: StyleSheetLocation::Shared,
                index: style_sheet_index,
            }))?
            .as_ref()
            .ok_or(StyleError::RemovedStyleSheet(NodeStyleSheetId {
                location: StyleSheetLocation::Shared,
                index: style_sheet_index,
            }))?;
        Ok(style_sheet_id.clone())
    }

    pub fn get_style_sheet_len(&self, location: StyleSheetLocation) -> StyleSheetIndex {
        (match location {
            StyleSheetLocation::Inline => self.inline_style_sheet.len(),
            StyleSheetLocation::Shared => self.shared_style_sheet_ids.len(),
        }) as _
    }
    pub fn push_applied_style_sheet(&mut self, applied_style_sheet: AppliedStyleSheet) {
        match applied_style_sheet {
            AppliedStyleSheet::None => {}
            AppliedStyleSheet::Inline(style_sheet) => {
                self.inline_style_sheet.push(Some(style_sheet));
            }
            AppliedStyleSheet::Shared(style_sheet_id) => {
                self.shared_style_sheet_ids.push(Some(style_sheet_id));
            }
        }
    }
    pub fn set_applied_style_sheet(
        &mut self,
        style_sheet_index: StyleSheetIndex,
        applied_style_sheet: AppliedStyleSheet,
    ) {
        match applied_style_sheet {
            AppliedStyleSheet::None => {
                self.inline_style_sheet[style_sheet_index as usize] = None;
            }
            AppliedStyleSheet::Inline(style_sheet_definition) => {
                self.inline_style_sheet[style_sheet_index as usize] = Some(style_sheet_definition);
            }
            AppliedStyleSheet::Shared(style_sheet_id) => {
                self.shared_style_sheet_ids[style_sheet_index as usize] = Some(style_sheet_id);
            }
        }
    }

    pub fn take_inline_style_sheets_from_member(
        &mut self,
        member_state: ApplyStyleSheetsMemberState,
    ) -> impl Iterator<Item = (StyleSheetIndex, StyleSheetDefinition)> + '_ {
        self.inline_style_sheet
            .iter_mut()
            .enumerate()
            .skip(member_state.inline_sheet_index as _)
            .take(member_state.inline_sheet_count as _)
            .filter_map(|n| n.1.take().map(|s| (n.0 as _, s)))
    }

    pub fn take_shared_style_sheets_from_member(
        &mut self,
        member_state: ApplyStyleSheetsMemberState,
    ) -> impl Iterator<Item = (StyleSheetIndex, StyleSheetId<BevyRenderer>)> + '_ {
        self.shared_style_sheet_ids
            .iter_mut()
            .enumerate()
            .skip(member_state.shared_sheet_index as _)
            .take(member_state.shared_sheet_count as _)
            .filter_map(|n| n.1.take().map(|s| (n.0 as _, s)))
    }
}

#[derive(Default)]
pub struct NodeStyleState {
    pub attr_infos: HashMap<StyleAttrId, NodeStyleAttrInfo>,
}

impl NodeStyleState {}

impl AttrStyleOwner for NodeStyleState {
    type ItemId = NodeAttrStyleItemId;

    fn from_definition_to_item_id(
        _style_sheet_definition: &StyleSheetDefinition,
        item_id: NodeAttrStyleItemId,
    ) -> Result<Self::ItemId> {
        Ok(item_id)
    }

    fn add_attr_style_item(
        &mut self,
        attr_style_item_id: NodeAttrStyleItemId,
        _entity_world_ref: EntityWorldRef,
    ) -> Result<()> {
        let value = match self.attr_infos.remove(&attr_style_item_id.attr_id) {
            None => attr_style_item_id.item_id.either_left().into(),
            Some(attr_info) => {
                let mut heap = attr_info
                    .0
                    .map_left(|item| {
                        let mut heap = BinaryHeap::new();
                        heap.push(item);
                        heap
                    })
                    .into_inner();
                heap.push(attr_style_item_id.item_id);
                heap.either_right().into()
            }
        };

        self.attr_infos.insert(attr_style_item_id.attr_id, value);
        Ok(())
    }

    fn remove_attr_style_item(&mut self, attr_style_item_id: NodeAttrStyleItemId) -> Result<bool> {
        match self.attr_infos.remove(&attr_style_item_id.attr_id) {
            None => Err(StyleError::NoFoundAttrId {
                attr_id: attr_style_item_id.attr_id,
            }),
            Some(value) => match value.0 {
                Either::Left(n) => {
                    assert_eq!(n, attr_style_item_id.item_id);
                    Ok(true)
                }
                Either::Right(mut heap) => {
                    let (result, heap) = if heap.peek() == Some(&attr_style_item_id.item_id) {
                        heap.pop();
                        (Ok(true), heap)
                    } else {
                        let prev_len = heap.len();
                        let heap = heap
                            .into_iter()
                            .filter(|n| n == &attr_style_item_id.item_id)
                            .collect::<BinaryHeap<NodeStyleItemId>>();
                        if heap.len() == prev_len {
                            return Err(StyleError::NoFoundStyleItemId {
                                item_id: attr_style_item_id.item_id,
                            });
                        }
                        (Ok(false), heap)
                    };
                    if !heap.is_empty() {
                        self.attr_infos
                            .insert(attr_style_item_id.attr_id, heap.either_right().into());
                    }
                    result
                }
            },
        }
    }

    fn check_style_sheet_type(&self, style_sheet_definition: &StyleSheetDefinition) -> Result<()> {
        if style_sheet_definition.interaction.is_some() {
            return Err(StyleError::StyleSheetTypeIncorrect);
        }
        Ok(())
    }
}

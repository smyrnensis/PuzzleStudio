use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    error::Error,
    fmt,
};

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, PostUpdate, ResMut, Resource};

use crate::{PuzzleBevyRendererSystems, PuzzleBevyViewId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BevyPublicationGroupId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BevyPublicationMember {
    pub view_id: PuzzleBevyViewId,
    pub generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BevyPublicationGroupError {
    GroupIdentityExhausted,
    UnknownGroup(BevyPublicationGroupId),
    GroupAlreadyRegistered(BevyPublicationGroupId),
    EmptyGroup(BevyPublicationGroupId),
    DuplicateMember(BevyPublicationMember),
    UnexpectedMember(BevyPublicationMember),
}

impl fmt::Display for BevyPublicationGroupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GroupIdentityExhausted => {
                write!(formatter, "renderer publication group identity exhausted")
            }
            Self::UnknownGroup(group) => {
                write!(formatter, "unknown renderer publication group {}", group.0)
            }
            Self::GroupAlreadyRegistered(group) => write!(
                formatter,
                "renderer publication group {} is already registered",
                group.0
            ),
            Self::EmptyGroup(group) => {
                write!(
                    formatter,
                    "renderer publication group {} must contain at least one viewport",
                    group.0
                )
            }
            Self::DuplicateMember(member) => write!(
                formatter,
                "renderer publication group repeats viewport {:?} generation {}",
                member.view_id, member.generation
            ),
            Self::UnexpectedMember(member) => write!(
                formatter,
                "viewport {:?} generation {} does not belong to its renderer publication group",
                member.view_id, member.generation
            ),
        }
    }
}

impl Error for BevyPublicationGroupError {}

#[derive(Default)]
struct PublicationGroup {
    expected: BTreeSet<BevyPublicationMember>,
    ready: BTreeSet<BevyPublicationMember>,
    materialized: BTreeSet<BevyPublicationMember>,
    published: BTreeSet<BevyPublicationMember>,
    authorized: bool,
    completion_queued: bool,
}

#[derive(Resource, Default)]
pub struct BevyPublicationGroups {
    next_group: u64,
    groups: BTreeMap<BevyPublicationGroupId, PublicationGroup>,
    member_groups: BTreeMap<BevyPublicationMember, BevyPublicationGroupId>,
    completed: VecDeque<BevyPublicationGroupId>,
}

impl BevyPublicationGroups {
    pub fn reserve_group(&mut self) -> Result<BevyPublicationGroupId, BevyPublicationGroupError> {
        self.next_group = self
            .next_group
            .checked_add(1)
            .ok_or(BevyPublicationGroupError::GroupIdentityExhausted)?;
        Ok(BevyPublicationGroupId(self.next_group))
    }

    pub fn register_group(
        &mut self,
        id: BevyPublicationGroupId,
        members: impl IntoIterator<Item = BevyPublicationMember>,
    ) -> Result<(), BevyPublicationGroupError> {
        if self.groups.contains_key(&id) {
            return Err(BevyPublicationGroupError::GroupAlreadyRegistered(id));
        }
        let mut expected = BTreeSet::new();
        for member in members {
            if self.member_groups.contains_key(&member) || !expected.insert(member.clone()) {
                return Err(BevyPublicationGroupError::DuplicateMember(member));
            }
        }
        if expected.is_empty() {
            return Err(BevyPublicationGroupError::EmptyGroup(id));
        }
        for member in &expected {
            self.member_groups.insert(member.clone(), id);
        }
        self.groups.insert(
            id,
            PublicationGroup {
                expected,
                ..Default::default()
            },
        );
        Ok(())
    }

    pub fn group_for(&self, member: &BevyPublicationMember) -> Option<BevyPublicationGroupId> {
        self.member_groups.get(member).copied()
    }

    pub fn mark_ready(
        &mut self,
        member: BevyPublicationMember,
    ) -> Result<(), BevyPublicationGroupError> {
        let group_id = self
            .group_for(&member)
            .ok_or_else(|| BevyPublicationGroupError::UnexpectedMember(member.clone()))?;
        let group = self
            .groups
            .get_mut(&group_id)
            .ok_or(BevyPublicationGroupError::UnknownGroup(group_id))?;
        if !group.expected.contains(&member) {
            return Err(BevyPublicationGroupError::UnexpectedMember(member));
        }
        group.ready.insert(member);
        Ok(())
    }

    pub fn mark_materialized(
        &mut self,
        member: BevyPublicationMember,
    ) -> Result<(), BevyPublicationGroupError> {
        let group_id = self
            .group_for(&member)
            .ok_or_else(|| BevyPublicationGroupError::UnexpectedMember(member.clone()))?;
        let group = self
            .groups
            .get_mut(&group_id)
            .ok_or(BevyPublicationGroupError::UnknownGroup(group_id))?;
        if !group.expected.contains(&member) {
            return Err(BevyPublicationGroupError::UnexpectedMember(member));
        }
        group.ready.insert(member.clone());
        group.materialized.insert(member);
        Ok(())
    }

    fn authorize_ready(&mut self) {
        let mut completed = Vec::new();
        for (id, group) in &mut self.groups {
            if group.ready == group.expected {
                group.authorized = true;
                group.published.extend(group.materialized.iter().cloned());
                if group.published == group.expected && !group.completion_queued {
                    group.completion_queued = true;
                    completed.push(*id);
                }
            }
        }
        self.completed.extend(completed);
    }

    pub fn is_authorized(&self, member: &BevyPublicationMember) -> bool {
        self.group_for(member)
            .and_then(|group| self.groups.get(&group))
            .is_some_and(|group| group.authorized)
    }

    pub fn mark_published(
        &mut self,
        member: BevyPublicationMember,
    ) -> Result<(), BevyPublicationGroupError> {
        let group_id = self
            .group_for(&member)
            .ok_or_else(|| BevyPublicationGroupError::UnexpectedMember(member.clone()))?;
        let group = self
            .groups
            .get_mut(&group_id)
            .ok_or(BevyPublicationGroupError::UnknownGroup(group_id))?;
        if !group.authorized || !group.expected.contains(&member) {
            return Err(BevyPublicationGroupError::UnexpectedMember(member));
        }
        group.published.insert(member);
        if group.published == group.expected && !group.completion_queued {
            group.completion_queued = true;
            self.completed.push_back(group_id);
        }
        Ok(())
    }

    pub fn cancel_group(
        &mut self,
        id: BevyPublicationGroupId,
    ) -> Result<(), BevyPublicationGroupError> {
        let group = self
            .groups
            .remove(&id)
            .ok_or(BevyPublicationGroupError::UnknownGroup(id))?;
        for member in group.expected {
            self.member_groups.remove(&member);
        }
        self.completed.retain(|completed| *completed != id);
        Ok(())
    }

    pub fn drain_completed(&mut self) -> impl Iterator<Item = BevyPublicationGroupId> + '_ {
        let completed = self.completed.drain(..).collect::<Vec<_>>();
        for id in &completed {
            if let Some(group) = self.groups.remove(id) {
                for member in group.expected {
                    self.member_groups.remove(&member);
                }
            }
        }
        completed.into_iter()
    }
}

#[derive(Default)]
pub struct PuzzleBevyPublicationPlugin;

impl Plugin for PuzzleBevyPublicationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BevyPublicationGroups>().add_systems(
            PostUpdate,
            authorize_ready_publication_groups
                .after(PuzzleBevyRendererSystems::ApplySubmittedFrames),
        );
    }
}

fn authorize_ready_publication_groups(mut groups: ResMut<BevyPublicationGroups>) {
    groups.authorize_ready();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(name: &str, generation: u64) -> BevyPublicationMember {
        BevyPublicationMember {
            view_id: PuzzleBevyViewId::two_d(name, "main"),
            generation,
        }
    }

    #[test]
    fn group_authorizes_and_completes_only_as_one_full_viewport_set() {
        let mut groups = BevyPublicationGroups::default();
        let id = groups.reserve_group().unwrap();
        let left = member("left", 3);
        let right = BevyPublicationMember {
            view_id: PuzzleBevyViewId::three_d("right", "main"),
            generation: 8,
        };
        groups
            .register_group(id, [left.clone(), right.clone()])
            .unwrap();

        groups.mark_ready(left.clone()).unwrap();
        assert!(!groups.is_authorized(&left));
        assert!(!groups.is_authorized(&right));
        groups.mark_ready(right.clone()).unwrap();
        groups.authorize_ready();
        assert!(groups.is_authorized(&left));
        assert!(groups.is_authorized(&right));

        groups.mark_published(right.clone()).unwrap();
        assert!(groups.drain_completed().next().is_none());
        groups.mark_published(left.clone()).unwrap();
        groups.authorize_ready();
        assert_eq!(groups.drain_completed().collect::<Vec<_>>(), vec![id]);
        assert!(groups.group_for(&left).is_none());
        assert!(groups.group_for(&right).is_none());
    }

    #[test]
    fn cancel_removes_every_member_of_the_group() {
        let mut groups = BevyPublicationGroups::default();
        let id = groups.reserve_group().unwrap();
        let left = member("left", 1);
        let right = member("right", 2);
        groups
            .register_group(id, [left.clone(), right.clone()])
            .unwrap();

        groups.cancel_group(id).unwrap();

        assert!(groups.group_for(&left).is_none());
        assert!(groups.group_for(&right).is_none());
        assert!(matches!(
            groups.mark_ready(left),
            Err(BevyPublicationGroupError::UnexpectedMember(_))
        ));
    }

    #[test]
    fn direct_materialization_completes_only_after_the_whole_group_is_ready() {
        let mut groups = BevyPublicationGroups::default();
        let id = groups.reserve_group().unwrap();
        let left = member("left", 1);
        let right = member("right", 2);
        groups
            .register_group(id, [left.clone(), right.clone()])
            .unwrap();

        groups.mark_materialized(left).unwrap();
        groups.authorize_ready();
        assert!(groups.drain_completed().next().is_none());

        groups.mark_materialized(right).unwrap();
        groups.authorize_ready();
        assert_eq!(groups.drain_completed().collect::<Vec<_>>(), vec![id]);
    }
}

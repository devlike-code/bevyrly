use bevy::{
    ecs::{
        component::Component,
        entity::Entity,
        system::{Commands, Query, Res},
    },
    hierarchy::DespawnRecursiveExt,
    prelude::{Deref, DerefMut},
    sprite::TextureAtlasSprite,
    time::{Time, Timer, TimerMode},
};

#[derive(Component)]
pub struct AnimationIndices {
    pub first: usize,
    pub last: usize,
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

pub fn animate_sprites(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &AnimationIndices,
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
    )>,
) {
    for (entity, indices, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            timer.reset();
            let mut next = sprite.index + 1;
            if next >= indices.last {
                if timer.mode() == TimerMode::Repeating {
                    next = indices.first;
                } else {
                    let _ = commands.get_entity(entity).map(|e| e.despawn_recursive());
                }
            }

            sprite.index = next;
        }
    }
}

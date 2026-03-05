//! 任務觸發點視覺效果

use bevy::prelude::*;
use std::collections::HashSet;
use std::f32::consts::FRAC_PI_2;

use super::super::story_data::StoryMissionId;
use super::super::story_manager::{StoryMissionDatabase, StoryMissionEvent, StoryMissionManager};
use super::super::trigger::{Trigger, TriggerAction, TriggerShape, TriggerType, TriggerVisual};

// --- 常數 ---
const TRIGGER_HEIGHT_OFFSET: f32 = 4.0;
const TRIGGER_PULSE_SPEED: f32 = 2.0;
const TRIGGER_FLOAT_AMPLITUDE: f32 = 0.3;
const TRIGGER_SCALE_AMPLITUDE: f32 = 0.1;

/// 世界任務觸發點標記
#[derive(Component)]
pub struct WorldMissionTrigger {
    pub mission_id: StoryMissionId,
    pub base_y: f32,
}

/// 任務觸發點視覺資源
#[derive(Resource)]
pub struct MissionTriggerVisuals {
    pub marker_mesh: Handle<Mesh>,
    pub marker_material: Handle<StandardMaterial>,
    pub ring_mesh: Handle<Mesh>,
    pub ring_material: Handle<StandardMaterial>,
}

/// 任務標記光環組件
#[derive(Component)]
pub struct MissionMarkerRing {
    pub rotation_speed: f32,
}

impl Default for MissionMarkerRing {
    fn default() -> Self {
        Self {
            rotation_speed: 1.0,
        }
    }
}

/// 設置任務觸發點視覺資源
pub fn setup_mission_trigger_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let visuals = MissionTriggerVisuals {
        marker_mesh: meshes.add(Cylinder::new(0.8, 8.0)),
        marker_material: materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.9, 0.0, 0.6),
            emissive: LinearRgba::new(1.0, 0.8, 0.0, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
        ring_mesh: meshes.add(Torus::new(1.5, 0.15)),
        ring_material: materials.add(StandardMaterial {
            base_color: Color::srgba(1.0, 0.9, 0.0, 0.4),
            emissive: LinearRgba::new(1.0, 0.7, 0.0, 1.0),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        }),
    };
    commands.insert_resource(visuals);
}

/// 生成可用任務的觸發點
pub fn spawn_mission_triggers(
    mut commands: Commands,
    manager: Res<StoryMissionManager>,
    database: Res<StoryMissionDatabase>,
    visuals: Res<MissionTriggerVisuals>,
    existing_triggers: Query<(Entity, &WorldMissionTrigger)>,
    mut mission_events: MessageReader<StoryMissionEvent>,
) {
    let mut missions_to_remove = HashSet::new();
    for event in mission_events.read() {
        match event {
            StoryMissionEvent::Started(mission_id)
            | StoryMissionEvent::Completed { mission_id, .. } => {
                missions_to_remove.insert(*mission_id);
            }
            _ => {}
        }
    }

    if !missions_to_remove.is_empty() {
        for (entity, trigger) in &existing_triggers {
            if missions_to_remove.contains(&trigger.mission_id) {
                commands.entity(entity).despawn();
                info!("移除任務 {} 的觸發點", trigger.mission_id);
            }
        }
    }

    let available_missions = manager.get_available_missions();

    for mission_id in available_missions {
        let already_exists = existing_triggers
            .iter()
            .any(|(_, t)| t.mission_id == mission_id);
        if already_exists {
            continue;
        }

        let Some(mission) = database.get(mission_id) else {
            continue;
        };
        let Some(trigger_pos) = mission.trigger_location else {
            continue;
        };

        spawn_mission_trigger_marker(
            &mut commands,
            &visuals,
            mission_id,
            trigger_pos,
            mission.trigger_radius,
            &mission.title,
        );

        info!(
            "生成任務觸發點: {} - {} 在 {:?}",
            mission_id, mission.title, trigger_pos
        );
    }
}

fn spawn_mission_trigger_marker(
    commands: &mut Commands,
    visuals: &MissionTriggerVisuals,
    mission_id: StoryMissionId,
    position: Vec3,
    radius: f32,
    title: &str,
) {
    let base_y = position.y + TRIGGER_HEIGHT_OFFSET;

    commands
        .spawn((
            Mesh3d(visuals.marker_mesh.clone()),
            MeshMaterial3d(visuals.marker_material.clone()),
            Transform::from_translation(position + Vec3::Y * TRIGGER_HEIGHT_OFFSET),
            WorldMissionTrigger { mission_id, base_y },
            Trigger::new(TriggerAction::Mission(mission_id))
                .with_shape(TriggerShape::Circle(radius))
                .with_type(TriggerType::OnInteract)
                .with_prompt(format!("按 F 開始任務: {title}")),
            TriggerVisual::default(),
            Name::new(format!("MissionTrigger_{mission_id}")),
        ))
        .with_children(|parent| {
            parent.spawn((
                Mesh3d(visuals.ring_mesh.clone()),
                MeshMaterial3d(visuals.ring_material.clone()),
                Transform::from_translation(Vec3::Y * -TRIGGER_HEIGHT_OFFSET)
                    .with_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                MissionMarkerRing::default(),
                Name::new(format!("MissionTriggerRing_{mission_id}")),
            ));
        });
}

/// 更新任務觸發點視覺效果
pub fn update_mission_trigger_visuals(
    mut trigger_query: Query<(&mut Transform, &WorldMissionTrigger)>,
    mut ring_query: Query<(&mut Transform, &MissionMarkerRing), Without<WorldMissionTrigger>>,
    time: Res<Time>,
) {
    let elapsed = time.elapsed_secs();
    let dt = time.delta_secs();

    for (mut transform, trigger) in &mut trigger_query {
        let phase = elapsed * TRIGGER_PULSE_SPEED;
        let offset = phase.sin() * TRIGGER_FLOAT_AMPLITUDE;
        transform.translation.y = trigger.base_y + offset;

        let scale_pulse = 1.0 + phase.sin().abs() * TRIGGER_SCALE_AMPLITUDE;
        transform.scale = Vec3::splat(scale_pulse);
    }

    for (mut transform, ring) in &mut ring_query {
        transform.rotate_y(dt * ring.rotation_speed);
    }
}

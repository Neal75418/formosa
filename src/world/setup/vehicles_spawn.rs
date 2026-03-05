//! 玩家與車輛生成

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;

use crate::core::{COLLISION_GROUP_CHARACTER, COLLISION_GROUP_STATIC, COLLISION_GROUP_VEHICLE};
use crate::player::Player;
use crate::vehicle::{spawn_scooter, VehicleModifications, VehiclePreset};
use crate::world::characters::spawn_player_character;
use crate::world::constants::{PLAYER_SPAWN_X, PLAYER_SPAWN_Z, X_KANGDING, Z_EMEI};
use crate::world::street_furniture::spawn_parking_garage;

/// 玩家、停車場、載具生成
pub(super) fn setup_player_and_vehicles(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    // 玩家生成：漢中街徒步區中央
    let start_pos = Vec3::new(PLAYER_SPAWN_X, 0.0, PLAYER_SPAWN_Z);

    spawn_player_character(
        commands,
        meshes,
        materials,
        start_pos,
        Player {
            speed: 8.0,
            rotation_speed: 3.0,
            ..default()
        },
    );

    // 峨嵋立體停車場
    spawn_parking_garage(
        commands,
        meshes,
        materials,
        Vec3::new(X_KANGDING + 25.0, 10.0, Z_EMEI + 20.0),
        22.0,
        22.0,
        32.0,
        "峨嵋停車場",
    );

    // === 共享載具材質與機車停放區 ===
    let vehicle_mats = crate::vehicle::VehicleMaterials::new(materials);
    commands.insert_resource(vehicle_mats.clone());

    // 漢中街徒步區旁 - 紅色機車
    spawn_scooter(
        commands,
        meshes,
        materials,
        &vehicle_mats,
        Vec3::new(12.0, 0.0, -8.0),
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
        Color::srgb(0.9, 0.1, 0.1),
    );

    // 徒步區閒置汽車 - 深藍色
    spawn_vehicle(
        commands,
        meshes,
        materials,
        Vec3::new(-8.0, 0.0, -15.0),
        VehiclePreset::car(),
        Color::srgb(0.2, 0.3, 0.6),
    );

    info!("🚗 已生成 1 台機車和 1 台汽車於徒步區");
}

#[allow(clippy::too_many_lines)]
fn spawn_vehicle(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    pos: Vec3,
    preset: VehiclePreset,
    color: Color,
) {
    use crate::vehicle::{
        VehicleHealth, VehicleId, VehiclePhysicsMode, VehicleType, VehicleVisualRoot,
    };

    let vehicle_type = preset.vehicle.vehicle_type;

    let (chassis_size, wheel_offset_z) = match vehicle_type {
        VehicleType::Car | VehicleType::Taxi | VehicleType::Scooter => {
            (Vec3::new(2.0, 0.6, 4.0), 1.2)
        }
        VehicleType::Bus => (Vec3::new(2.8, 1.2, 8.0), 2.5),
    };

    commands
        .spawn((
            Transform::from_translation(pos + Vec3::new(0.0, 0.5, 0.0)),
            GlobalTransform::default(),
            Visibility::default(),
            Collider::cuboid(chassis_size.x / 2.0, 0.75, chassis_size.z / 2.0),
            VehicleHealth::for_vehicle_type(vehicle_type),
            VehicleId::new(),
            VehicleModifications::default(),
            preset.into_components(),
            RigidBody::KinematicPositionBased,
            VehiclePhysicsMode::Kinematic,
            CollisionGroups::new(
                COLLISION_GROUP_VEHICLE,
                COLLISION_GROUP_CHARACTER | COLLISION_GROUP_VEHICLE | COLLISION_GROUP_STATIC,
            ),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Transform::default(),
                    GlobalTransform::default(),
                    Visibility::default(),
                    VehicleVisualRoot,
                ))
                .with_children(|parent| {
                    // A. 底盤
                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::from_size(chassis_size))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: color,
                            perceptual_roughness: 0.3,
                            metallic: 0.5,
                            ..default()
                        })),
                        Transform::from_xyz(0.0, 0.0, 0.0),
                        GlobalTransform::default(),
                    ));

                    // B. 車艙
                    let is_bus = chassis_size.x > 2.5;
                    let cabin_size = if is_bus {
                        Vec3::new(2.7, 1.0, 7.5)
                    } else {
                        Vec3::new(1.8, 0.5, 2.0)
                    };

                    let cabin_y = chassis_size.y / 2.0 + cabin_size.y / 2.0;
                    let cabin_z_offset = if is_bus { 0.0 } else { -0.2 };

                    parent.spawn((
                        Mesh3d(meshes.add(Cuboid::from_size(cabin_size))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.1, 0.1, 0.1),
                            perceptual_roughness: 0.1,
                            metallic: 0.8,
                            ..default()
                        })),
                        Transform::from_xyz(0.0, cabin_y, cabin_z_offset),
                        GlobalTransform::default(),
                    ));

                    // C. 輪子
                    let wheel_mesh = meshes.add(Cylinder::new(0.35, 0.3));
                    let wheel_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(0.0, 0.0, 0.0),
                        perceptual_roughness: 0.9,
                        ..default()
                    });

                    let wheel_y = -chassis_size.y / 2.0;
                    let wheel_x = chassis_size.x / 2.0;

                    let wheel_positions = [
                        Vec3::new(-wheel_x, wheel_y, -wheel_offset_z),
                        Vec3::new(wheel_x, wheel_y, -wheel_offset_z),
                        Vec3::new(-wheel_x, wheel_y, wheel_offset_z),
                        Vec3::new(wheel_x, wheel_y, wheel_offset_z),
                    ];

                    for pos in wheel_positions {
                        parent.spawn((
                            Mesh3d(wheel_mesh.clone()),
                            MeshMaterial3d(wheel_mat.clone()),
                            Transform::from_translation(pos)
                                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
                            GlobalTransform::default(),
                        ));
                    }

                    // D. 車燈
                    let headlight_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(1.0, 1.0, 1.0),
                        emissive: LinearRgba::new(20.0, 18.0, 10.0, 1.0),
                        ..default()
                    });

                    let light_x = chassis_size.x / 2.0 - 0.4;
                    let light_mesh = meshes.add(Cuboid::new(0.4, 0.2, 0.1));

                    let taillight_mat = materials.add(StandardMaterial {
                        base_color: Color::srgb(1.0, 0.0, 0.0),
                        emissive: LinearRgba::new(15.0, 0.0, 0.0, 1.0),
                        ..default()
                    });

                    let lights: [(Handle<StandardMaterial>, f32); 2] = [
                        (headlight_mat, -chassis_size.z / 2.0 - 0.05),
                        (taillight_mat, chassis_size.z / 2.0 + 0.05),
                    ];
                    for (mat, z) in lights {
                        for x in [-light_x, light_x] {
                            parent.spawn((
                                Mesh3d(light_mesh.clone()),
                                MeshMaterial3d(mat.clone()),
                                Transform::from_xyz(x, 0.1, z),
                                GlobalTransform::default(),
                            ));
                        }
                    }
                });
        });
}

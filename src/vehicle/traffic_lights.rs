//! 紅綠燈交通系統
#![allow(dead_code)]

use bevy::prelude::*;

// ============================================================================
// 紅綠燈類型定義
// ============================================================================

/// 紅綠燈狀態
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TrafficLightState {
    /// 紅燈 - 停止
    Red,
    /// 黃燈 - 準備停止
    Yellow,
    /// 綠燈 - 通行
    #[default]
    Green,
}

impl TrafficLightState {
    /// 取得下一個狀態
    pub fn next(&self) -> Self {
        match self {
            TrafficLightState::Green => TrafficLightState::Yellow,
            TrafficLightState::Yellow => TrafficLightState::Red,
            TrafficLightState::Red => TrafficLightState::Green,
        }
    }

    /// 取得狀態持續時間（秒）
    pub fn duration(&self) -> f32 {
        match self {
            TrafficLightState::Green => 8.0,   // 綠燈 8 秒
            TrafficLightState::Yellow => 2.0,  // 黃燈 2 秒
            TrafficLightState::Red => 10.0,    // 紅燈 10 秒
        }
    }

    /// 取得燈光顏色
    pub fn color(&self) -> Color {
        match self {
            TrafficLightState::Green => Color::srgb(0.0, 1.0, 0.0),
            TrafficLightState::Yellow => Color::srgb(1.0, 0.9, 0.0),
            TrafficLightState::Red => Color::srgb(1.0, 0.0, 0.0),
        }
    }

    /// 取得發光顏色（用於燈泡）
    pub fn emissive(&self) -> LinearRgba {
        match self {
            TrafficLightState::Green => LinearRgba::new(0.0, 15.0, 0.0, 1.0),
            TrafficLightState::Yellow => LinearRgba::new(15.0, 13.0, 0.0, 1.0),
            TrafficLightState::Red => LinearRgba::new(15.0, 0.0, 0.0, 1.0),
        }
    }
}

/// 紅綠燈組件
#[derive(Component)]
pub struct TrafficLight {
    /// 當前狀態
    pub state: TrafficLightState,
    /// 狀態計時器
    pub timer: Timer,
    /// 控制方向（車輛前進方向需與此方向一致才受此燈控制）
    /// 通常是燈面向的方向
    pub control_direction: Vec3,
    /// 偵測範圍（NPC 車輛在此範圍內會看到紅燈）
    pub detection_range: f32,
    /// 是否為主燈（主燈和副燈狀態相反）
    pub is_primary: bool,
}

impl Default for TrafficLight {
    fn default() -> Self {
        Self {
            state: TrafficLightState::Green,
            timer: Timer::from_seconds(TrafficLightState::Green.duration(), TimerMode::Once),
            control_direction: Vec3::NEG_Z,  // 默認面向 -Z
            detection_range: 15.0,
            is_primary: true,
        }
    }
}

impl TrafficLight {
    /// 創建指定方向的紅綠燈
    pub fn new(direction: Vec3, is_primary: bool) -> Self {
        let initial_state = if is_primary {
            TrafficLightState::Green
        } else {
            TrafficLightState::Red  // 副燈初始為紅燈
        };
        Self {
            state: initial_state,
            timer: Timer::from_seconds(initial_state.duration(), TimerMode::Once),
            control_direction: direction.normalize_or_zero(),
            detection_range: 15.0,
            is_primary,
        }
    }

    /// 切換到下一個狀態
    pub fn advance(&mut self) {
        self.state = self.state.next();
        self.timer = Timer::from_seconds(self.state.duration(), TimerMode::Once);
    }

    /// 檢查車輛是否應該停止
    /// - 車輛位置在偵測範圍內
    /// - 車輛行駛方向與控制方向大致相同
    pub fn should_vehicle_stop(&self, vehicle_pos: Vec3, vehicle_forward: Vec3, light_pos: Vec3) -> bool {
        // 只有紅燈需要停止
        if self.state != TrafficLightState::Red {
            return false;
        }

        // 檢查距離
        let to_light = light_pos - vehicle_pos;
        let distance = to_light.length();
        if distance > self.detection_range || distance < 2.0 {
            return false;  // 太遠或已經過燈
        }

        // 檢查車輛是否面向燈（車輛往燈的方向行駛）
        let to_light_flat = Vec3::new(to_light.x, 0.0, to_light.z).normalize_or_zero();
        let vehicle_forward_flat = Vec3::new(vehicle_forward.x, 0.0, vehicle_forward.z).normalize_or_zero();

        // 車輛需要朝向燈的方向（點積 > 0.5，約 60 度內）
        let dot_to_light = vehicle_forward_flat.dot(to_light_flat);
        if dot_to_light < 0.5 {
            return false;
        }

        // 檢查車輛行駛方向是否受此燈控制
        // 車輛前進方向需要與燈的控制方向相反（車輛朝向燈）
        let dot_control = vehicle_forward_flat.dot(-self.control_direction);
        dot_control > 0.5
    }
}

/// 紅綠燈燈泡標記（用於更新發光顏色）
#[derive(Component)]
pub struct TrafficLightBulb {
    /// 對應的燈光狀態（紅/黃/綠）
    pub light_type: TrafficLightState,
}

/// 紅綠燈視覺效果資源
#[derive(Resource)]
pub struct TrafficLightVisuals {
    /// 燈柱 mesh
    pub pole_mesh: Handle<Mesh>,
    /// 燈柱材質
    pub pole_material: Handle<StandardMaterial>,
    /// 燈箱 mesh
    pub box_mesh: Handle<Mesh>,
    /// 燈箱材質
    pub box_material: Handle<StandardMaterial>,
    /// 燈泡 mesh
    pub bulb_mesh: Handle<Mesh>,
    /// 紅燈材質（亮）
    pub red_on_material: Handle<StandardMaterial>,
    /// 紅燈材質（暗）
    pub red_off_material: Handle<StandardMaterial>,
    /// 黃燈材質（亮）
    pub yellow_on_material: Handle<StandardMaterial>,
    /// 黃燈材質（暗）
    pub yellow_off_material: Handle<StandardMaterial>,
    /// 綠燈材質（亮）
    pub green_on_material: Handle<StandardMaterial>,
    /// 綠燈材質（暗）
    pub green_off_material: Handle<StandardMaterial>,
}

impl TrafficLightVisuals {
    /// 建立新實例
    pub fn new(meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>) -> Self {
        Self {
            pole_mesh: meshes.add(Cylinder::new(0.1, 4.0)),
            pole_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.3, 0.3),
                metallic: 0.8,
                perceptual_roughness: 0.6,
                ..default()
            }),
            box_mesh: meshes.add(Cuboid::new(0.5, 1.2, 0.3)),
            box_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.15, 0.15, 0.15),
                metallic: 0.5,
                perceptual_roughness: 0.8,
                ..default()
            }),
            bulb_mesh: meshes.add(Sphere::new(0.12)),
            red_on_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                emissive: LinearRgba::new(15.0, 0.0, 0.0, 1.0),
                ..default()
            }),
            red_off_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.1, 0.1),
                ..default()
            }),
            yellow_on_material: materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.9, 0.0),
                emissive: LinearRgba::new(15.0, 13.0, 0.0, 1.0),
                ..default()
            }),
            yellow_off_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.27, 0.1),
                ..default()
            }),
            green_on_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.0, 1.0, 0.0),
                emissive: LinearRgba::new(0.0, 15.0, 0.0, 1.0),
                ..default()
            }),
            green_off_material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.1, 0.3, 0.1),
                ..default()
            }),
        }
    }

    /// 根據當前狀態取得燈泡材質
    pub fn get_bulb_material(&self, bulb_type: TrafficLightState, current_state: TrafficLightState) -> Handle<StandardMaterial> {
        let is_on = bulb_type == current_state;
        match bulb_type {
            TrafficLightState::Red => if is_on { self.red_on_material.clone() } else { self.red_off_material.clone() },
            TrafficLightState::Yellow => if is_on { self.yellow_on_material.clone() } else { self.yellow_off_material.clone() },
            TrafficLightState::Green => if is_on { self.green_on_material.clone() } else { self.green_off_material.clone() },
        }
    }
}

// ============================================================================
// 紅綠燈交通系統
// ============================================================================

/// 初始化紅綠燈視覺效果資源
pub fn setup_traffic_lights(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(TrafficLightVisuals::new(&mut meshes, &mut materials));
    info!("🚦 紅綠燈系統已初始化");
}

/// 紅綠燈狀態更新系統
/// 處理紅綠燈的循環切換
pub fn traffic_light_cycle_system(
    time: Res<Time>,
    mut traffic_lights: Query<(Entity, &mut TrafficLight, &Children)>,
    mut bulb_query: Query<(&TrafficLightBulb, &mut MeshMaterial3d<StandardMaterial>)>,
    visuals: Option<Res<TrafficLightVisuals>>,
) {
    let Some(visuals) = visuals else { return };

    for (_entity, mut light, children) in traffic_lights.iter_mut() {
        // 更新計時器
        light.timer.tick(time.delta());

        // 計時器結束時切換狀態
        if light.timer.just_finished() {
            light.advance();

            // 更新燈泡材質
            for child in children.iter() {
                if let Ok((bulb, mut material)) = bulb_query.get_mut(child) {
                    *material =
                        MeshMaterial3d(visuals.get_bulb_material(bulb.light_type, light.state));
                }
            }
        }
    }
}

/// 生成紅綠燈實體
pub fn spawn_traffic_light(
    commands: &mut Commands,
    visuals: &TrafficLightVisuals,
    position: Vec3,
    direction: Vec3,
    is_primary: bool,
) {
    let rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));
    let initial_state = if is_primary {
        TrafficLightState::Green
    } else {
        TrafficLightState::Red
    };

    commands
        .spawn((
            // 空間組件
            Transform {
                translation: position,
                rotation,
                ..default()
            },
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::default(),
            ViewVisibility::default(),
            // 紅綠燈組件
            TrafficLight::new(direction, is_primary),
            Name::new("TrafficLight"),
        ))
        .with_children(|parent| {
            // 燈柱
            parent.spawn((
                Mesh3d(visuals.pole_mesh.clone()),
                MeshMaterial3d(visuals.pole_material.clone()),
                Transform::from_xyz(0.0, 2.0, 0.0),
                GlobalTransform::default(),
            ));

            // 燈箱
            parent.spawn((
                Mesh3d(visuals.box_mesh.clone()),
                MeshMaterial3d(visuals.box_material.clone()),
                Transform::from_xyz(0.0, 4.5, 0.0),
                GlobalTransform::default(),
            ));

            // 紅燈（頂部）
            parent.spawn((
                Mesh3d(visuals.bulb_mesh.clone()),
                MeshMaterial3d(visuals.get_bulb_material(TrafficLightState::Red, initial_state)),
                Transform::from_xyz(0.0, 4.9, 0.16),
                GlobalTransform::default(),
                TrafficLightBulb {
                    light_type: TrafficLightState::Red,
                },
            ));

            // 黃燈（中間）
            parent.spawn((
                Mesh3d(visuals.bulb_mesh.clone()),
                MeshMaterial3d(visuals.get_bulb_material(TrafficLightState::Yellow, initial_state)),
                Transform::from_xyz(0.0, 4.5, 0.16),
                GlobalTransform::default(),
                TrafficLightBulb {
                    light_type: TrafficLightState::Yellow,
                },
            ));

            // 綠燈（底部）
            parent.spawn((
                Mesh3d(visuals.bulb_mesh.clone()),
                MeshMaterial3d(visuals.get_bulb_material(TrafficLightState::Green, initial_state)),
                Transform::from_xyz(0.0, 4.1, 0.16),
                GlobalTransform::default(),
                TrafficLightBulb {
                    light_type: TrafficLightState::Green,
                },
            ));
        });
}

/// 生成交叉路口的紅綠燈組（4個方向）
/// ns_road_width: 南北向道路寬度（X方向）
/// ew_road_width: 東西向道路寬度（Z方向）
pub fn spawn_intersection_lights(
    commands: &mut Commands,
    visuals: &TrafficLightVisuals,
    center: Vec3,
    ns_road_width: f32,
    ew_road_width: f32,
) {
    // 紅綠燈放在道路邊緣外側 1 公尺
    let offset_x = ns_road_width / 2.0 + 1.0; // X 方向偏移（南北向道路寬度）
    let offset_z = ew_road_width / 2.0 + 1.0; // Z 方向偏移（東西向道路寬度）

    // 北向（控制南北向車流）- 主燈
    // 放在交叉口西北角（西側人行道，面向南來車）
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(-offset_x, 0.0, -offset_z),
        Vec3::NEG_Z,
        true,
    );

    // 南向（控制南北向車流）- 主燈
    // 放在交叉口東南角（東側人行道，面向北來車）
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(offset_x, 0.0, offset_z),
        Vec3::Z,
        true,
    );

    // 東向（控制東西向車流）- 副燈
    // 放在交叉口東北角（北側人行道，面向西來車）
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(offset_x, 0.0, -offset_z),
        Vec3::X,
        false,
    );

    // 西向（控制東西向車流）- 副燈
    // 放在交叉口西南角（南側人行道，面向東來車）
    spawn_traffic_light(
        commands,
        visuals,
        center + Vec3::new(-offset_x, 0.0, offset_z),
        Vec3::NEG_X,
        false,
    );
}

/// 在世界中生成紅綠燈（西門町主要路口）
/// 此系統需要在 setup_traffic_lights 之後執行
pub fn spawn_world_traffic_lights(
    mut commands: Commands,
    visuals: Option<Res<TrafficLightVisuals>>,
) {
    let Some(visuals) = visuals else {
        warn!("TrafficLightVisuals 資源不存在，無法生成紅綠燈");
        return;
    };

    info!("🚦 正在生成交通燈...");

    // 道路常數（與 setup.rs 一致）
    // 南北向道路 X 位置
    const X_ZHONGHUA: f32 = 80.0; // 中華路
    const X_XINING: f32 = -55.0; // 西寧南路
                                 // 東西向道路 Z 位置
    const Z_HANKOU: f32 = -80.0; // 漢口街
    const Z_CHENGDU: f32 = 50.0; // 成都路
                                 // 道路寬度
    const W_ZHONGHUA: f32 = 40.0; // 中華路寬度
    const W_MAIN: f32 = 16.0; // 成都路寬度
    const W_SECONDARY: f32 = 12.0; // 西寧路、漢口街寬度

    // 主要路口：(位置, 南北道路寬度, 東西道路寬度)
    let intersections: [(Vec3, f32, f32); 4] = [
        // 西寧路/成都路交叉口
        (Vec3::new(X_XINING, 0.0, Z_CHENGDU), W_SECONDARY, W_MAIN),
        // 中華路/成都路交叉口
        (Vec3::new(X_ZHONGHUA, 0.0, Z_CHENGDU), W_ZHONGHUA, W_MAIN),
        // 西寧路/漢口街交叉口
        (Vec3::new(X_XINING, 0.0, Z_HANKOU), W_SECONDARY, W_SECONDARY),
        // 中華路/漢口街交叉口
        (
            Vec3::new(X_ZHONGHUA, 0.0, Z_HANKOU),
            W_ZHONGHUA,
            W_SECONDARY,
        ),
    ];

    for (center, ns_width, ew_width) in intersections.iter() {
        spawn_intersection_lights(&mut commands, &visuals, *center, *ns_width, *ew_width);
    }

    info!(
        "✅ 已生成 {} 組交通燈（共 {} 個）",
        intersections.len(),
        intersections.len() * 4
    );
}

// ============================================================================
// 單元測試
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- TrafficLightState ---

    #[test]
    fn traffic_light_state_cycle() {
        let g = TrafficLightState::Green;
        let y = g.next();
        let r = y.next();
        let g2 = r.next();
        assert_eq!(y, TrafficLightState::Yellow);
        assert_eq!(r, TrafficLightState::Red);
        assert_eq!(g2, TrafficLightState::Green);
    }

    #[test]
    fn traffic_light_state_durations() {
        assert_eq!(TrafficLightState::Green.duration(), 8.0);
        assert_eq!(TrafficLightState::Yellow.duration(), 2.0);
        assert_eq!(TrafficLightState::Red.duration(), 10.0);
    }

    // --- TrafficLight ---

    #[test]
    fn traffic_light_advance_cycles() {
        let mut light = TrafficLight::new(Vec3::NEG_Z, true);
        assert_eq!(light.state, TrafficLightState::Green);
        light.advance();
        assert_eq!(light.state, TrafficLightState::Yellow);
        light.advance();
        assert_eq!(light.state, TrafficLightState::Red);
    }

    #[test]
    fn traffic_light_should_stop_red_facing() {
        // control_direction=NEG_Z → 車輛需反向（+Z）才受控
        let mut light = TrafficLight::new(Vec3::NEG_Z, true);
        light.state = TrafficLightState::Red;
        let light_pos = Vec3::new(0.0, 0.0, 10.0);
        let vehicle_pos = Vec3::ZERO;
        let vehicle_forward = Vec3::Z; // 朝向燈
        assert!(light.should_vehicle_stop(vehicle_pos, vehicle_forward, light_pos));
    }

    #[test]
    fn traffic_light_should_not_stop_green() {
        let light = TrafficLight::new(Vec3::NEG_Z, true);
        let light_pos = Vec3::new(0.0, 0.0, 10.0);
        assert!(!light.should_vehicle_stop(Vec3::ZERO, Vec3::Z, light_pos));
    }
}

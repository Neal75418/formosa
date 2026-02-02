//! 車輛改裝系統
//!
//! 允許玩家購買和安裝車輛改裝，提升性能
#![allow(dead_code)] // 預留功能：此檔案包含已定義但尚未整合的功能

use bevy::prelude::*;

// ============================================================================
// 常數
// ============================================================================

/// 氮氣加速倍率
pub const NITRO_BOOST_MULTIPLIER: f32 = 1.5;
/// 氮氣消耗速率（每秒）
pub const NITRO_DRAIN_RATE: f32 = 0.2;
/// 氮氣回充速率（每秒）
pub const NITRO_RECHARGE_RATE: f32 = 0.05;
/// 氮氣價格
pub const NITRO_PRICE: i32 = 25_000;

// ============================================================================
// 改裝類別
// ============================================================================

/// 改裝類別
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModCategory {
    Engine,       // 引擎：加速度 +
    Transmission, // 變速箱：最高速度 +
    Suspension,   // 懸吊：操控性 +
    Brakes,       // 煞車：煞車力 +
    Tires,        // 輪胎：抓地力 +
    Armor,        // 裝甲：耐久度 +
}

impl ModCategory {
    /// 取得類別名稱
    pub fn name(&self) -> &'static str {
        match self {
            ModCategory::Engine => "引擎",
            ModCategory::Transmission => "變速箱",
            ModCategory::Suspension => "懸吊",
            ModCategory::Brakes => "煞車",
            ModCategory::Tires => "輪胎",
            ModCategory::Armor => "裝甲",
        }
    }

    /// 取得類別描述
    pub fn description(&self) -> &'static str {
        match self {
            ModCategory::Engine => "提升加速度",
            ModCategory::Transmission => "提升最高速度",
            ModCategory::Suspension => "提升操控性",
            ModCategory::Brakes => "提升煞車力",
            ModCategory::Tires => "提升抓地力",
            ModCategory::Armor => "提升耐久度",
        }
    }

    /// 取得所有類別
    pub fn all() -> &'static [ModCategory] {
        &[
            ModCategory::Engine,
            ModCategory::Transmission,
            ModCategory::Suspension,
            ModCategory::Brakes,
            ModCategory::Tires,
            ModCategory::Armor,
        ]
    }
}

// ============================================================================
// 改裝等級
// ============================================================================

/// 改裝等級
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ModLevel {
    #[default]
    Stock,  // 原廠
    Level1, // 一級改裝
    Level2, // 二級改裝
    Level3, // 三級改裝
}

impl ModLevel {
    /// 取得數值倍率
    pub fn multiplier(&self) -> f32 {
        match self {
            ModLevel::Stock => 1.0,
            ModLevel::Level1 => 1.10,
            ModLevel::Level2 => 1.25,
            ModLevel::Level3 => 1.50,
        }
    }

    /// 取得升級價格
    pub fn price(&self) -> i32 {
        match self {
            ModLevel::Stock => 0,
            ModLevel::Level1 => 5_000,
            ModLevel::Level2 => 15_000,
            ModLevel::Level3 => 40_000,
        }
    }

    /// 取得等級名稱
    pub fn name(&self) -> &'static str {
        match self {
            ModLevel::Stock => "原廠",
            ModLevel::Level1 => "一級",
            ModLevel::Level2 => "二級",
            ModLevel::Level3 => "三級",
        }
    }

    /// 取得下一級
    pub fn next(&self) -> Option<ModLevel> {
        match self {
            ModLevel::Stock => Some(ModLevel::Level1),
            ModLevel::Level1 => Some(ModLevel::Level2),
            ModLevel::Level2 => Some(ModLevel::Level3),
            ModLevel::Level3 => None,
        }
    }

    /// 取得升級到下一級的價格
    pub fn upgrade_price(&self) -> Option<i32> {
        self.next().map(|level| level.price())
    }
}

// ============================================================================
// 車輛改裝組件
// ============================================================================

/// 車輛改裝狀態組件
#[derive(Component, Default, Clone, Debug)]
pub struct VehicleModifications {
    /// 引擎等級
    pub engine: ModLevel,
    /// 變速箱等級
    pub transmission: ModLevel,
    /// 懸吊等級
    pub suspension: ModLevel,
    /// 煞車等級
    pub brakes: ModLevel,
    /// 輪胎等級
    pub tires: ModLevel,
    /// 裝甲等級
    pub armor: ModLevel,
    /// 是否安裝氮氣加速
    pub has_nitro: bool,
    /// 氮氣充能量 (0.0 - 1.0)
    pub nitro_charge: f32,
}

impl VehicleModifications {
    /// 取得指定類別的等級
    pub fn get_level(&self, category: ModCategory) -> ModLevel {
        match category {
            ModCategory::Engine => self.engine,
            ModCategory::Transmission => self.transmission,
            ModCategory::Suspension => self.suspension,
            ModCategory::Brakes => self.brakes,
            ModCategory::Tires => self.tires,
            ModCategory::Armor => self.armor,
        }
    }

    /// 設定指定類別的等級
    pub fn set_level(&mut self, category: ModCategory, level: ModLevel) {
        match category {
            ModCategory::Engine => self.engine = level,
            ModCategory::Transmission => self.transmission = level,
            ModCategory::Suspension => self.suspension = level,
            ModCategory::Brakes => self.brakes = level,
            ModCategory::Tires => self.tires = level,
            ModCategory::Armor => self.armor = level,
        }
    }

    /// 升級指定類別
    pub fn upgrade(&mut self, category: ModCategory) -> bool {
        let current = self.get_level(category);
        if let Some(next) = current.next() {
            self.set_level(category, next);
            true
        } else {
            false
        }
    }

    /// 取得改裝後的數值倍率
    pub fn get_multiplier(&self, category: ModCategory) -> f32 {
        self.get_level(category).multiplier()
    }

    /// 計算所有改裝的總價值
    pub fn total_value(&self) -> i32 {
        let mut total = 0;
        for category in ModCategory::all() {
            total += self.get_level(*category).price();
        }
        if self.has_nitro {
            total += NITRO_PRICE;
        }
        total
    }
}

// ============================================================================
// 氮氣加速組件
// ============================================================================

/// 氮氣加速狀態組件
#[derive(Component, Default)]
pub struct NitroBoost {
    /// 是否正在使用
    pub is_active: bool,
    /// 加速倍率
    pub boost_multiplier: f32,
}

impl NitroBoost {
    pub fn new() -> Self {
        Self {
            is_active: false,
            boost_multiplier: NITRO_BOOST_MULTIPLIER,
        }
    }
}

// ============================================================================
// 改裝後數值計算
// ============================================================================

/// 計算改裝後的加速度
pub fn modified_acceleration(base: f32, mods: &VehicleModifications) -> f32 {
    base * mods.engine.multiplier()
}

/// 計算改裝後的最高速度
pub fn modified_max_speed(base: f32, mods: &VehicleModifications) -> f32 {
    base * mods.transmission.multiplier()
}

/// 計算改裝後的操控性
pub fn modified_handling(base: f32, mods: &VehicleModifications) -> f32 {
    base * mods.suspension.multiplier()
}

/// 計算改裝後的煞車力
pub fn modified_brake_power(base: f32, mods: &VehicleModifications) -> f32 {
    base * mods.brakes.multiplier()
}

/// 計算改裝後的抓地力
pub fn modified_grip(base: f32, mods: &VehicleModifications) -> f32 {
    base * mods.tires.multiplier()
}

/// 計算改裝後的耐久度
pub fn modified_health(base: f32, mods: &VehicleModifications) -> f32 {
    base * mods.armor.multiplier()
}

// ============================================================================
// 事件
// ============================================================================

/// 購買改裝事件
#[derive(Message)]
pub struct PurchaseModificationEvent {
    /// 車輛實體
    pub vehicle: Entity,
    /// 改裝類別
    pub category: ModCategory,
}

/// 購買氮氣事件
#[derive(Message)]
pub struct PurchaseNitroEvent {
    /// 車輛實體
    pub vehicle: Entity,
}

/// 改裝完成事件
#[derive(Message)]
pub struct ModificationCompleteEvent {
    /// 車輛實體
    pub vehicle: Entity,
    /// 改裝類別
    pub category: ModCategory,
    /// 新等級
    pub new_level: ModLevel,
}

// ============================================================================
// 系統
// ============================================================================

/// 處理改裝購買事件
pub fn purchase_modification_system(
    mut events: MessageReader<PurchaseModificationEvent>,
    mut complete_events: MessageWriter<ModificationCompleteEvent>,
    mut vehicle_query: Query<(&mut VehicleModifications, Option<&mut super::VehicleHealth>)>,
    mut wallet: ResMut<crate::economy::PlayerWallet>,
) {
    for event in events.read() {
        let Ok((mut mods, health)) = vehicle_query.get_mut(event.vehicle) else {
            warn!("Vehicle {:?} not found for modification", event.vehicle);
            continue;
        };

        let current_level = mods.get_level(event.category);
        let Some(next_level) = current_level.next() else {
            info!("已達最高等級: {:?}", event.category);
            continue;
        };

        let price = next_level.price();

        // 扣款並升級（spend_cash 會檢查餘額並追蹤 total_spent）
        if !wallet.spend_cash(price) {
            info!("餘額不足: 需要 ${}, 現有 ${}", price, wallet.cash);
            continue;
        }
        mods.upgrade(event.category);

        // 裝甲改裝：增加車輛最大血量
        if event.category == ModCategory::Armor {
            if let Some(mut vehicle_health) = health {
                // 計算增量倍率（新等級 / 舊等級）
                let incremental_multiplier = next_level.multiplier() / current_level.multiplier();
                vehicle_health.apply_armor_upgrade(incremental_multiplier);
                info!(
                    "裝甲升級: 血量 {} -> {} ({}x)",
                    vehicle_health.max / incremental_multiplier,
                    vehicle_health.max,
                    incremental_multiplier
                );
            }
        }

        info!(
            "購買改裝: {:?} -> {} (${price})",
            event.category,
            next_level.name()
        );

        complete_events.write(ModificationCompleteEvent {
            vehicle: event.vehicle,
            category: event.category,
            new_level: next_level,
        });
    }
}

/// 處理氮氣購買事件
pub fn purchase_nitro_system(
    mut events: MessageReader<PurchaseNitroEvent>,
    mut vehicle_query: Query<(&mut VehicleModifications, Option<&mut NitroBoost>)>,
    mut commands: Commands,
    mut wallet: ResMut<crate::economy::PlayerWallet>,
) {
    for event in events.read() {
        let Ok((mut mods, nitro)) = vehicle_query.get_mut(event.vehicle) else {
            warn!("Vehicle {:?} not found for nitro", event.vehicle);
            continue;
        };

        if mods.has_nitro {
            info!("已安裝氮氣加速");
            continue;
        }

        // 扣款並安裝（spend_cash 會檢查餘額並追蹤 total_spent）
        if !wallet.spend_cash(NITRO_PRICE) {
            info!("餘額不足: 需要 ${}, 現有 ${}", NITRO_PRICE, wallet.cash);
            continue;
        }
        mods.has_nitro = true;
        mods.nitro_charge = 1.0;

        // 添加 NitroBoost 組件
        if nitro.is_none() {
            commands.entity(event.vehicle).insert(NitroBoost::new());
        }

        info!("購買氮氣加速 (${NITRO_PRICE})");
    }
}

/// 氮氣加速系統
pub fn nitro_boost_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut VehicleModifications, &mut NitroBoost)>,
    game_state: Res<crate::core::GameState>,
) {
    // 只有在駕駛車輛時才處理
    if !game_state.player_in_vehicle {
        return;
    }

    let dt = time.delta_secs();

    for (mut mods, mut nitro) in query.iter_mut() {
        if !mods.has_nitro {
            continue;
        }

        // Shift 鍵（左或右）啟動氮氣
        let wants_boost = keyboard.pressed(KeyCode::ShiftLeft)
            || keyboard.pressed(KeyCode::ShiftRight);

        if wants_boost && mods.nitro_charge > 0.0 {
            nitro.is_active = true;
            mods.nitro_charge = (mods.nitro_charge - NITRO_DRAIN_RATE * dt).max(0.0);
        } else {
            nitro.is_active = false;
            // 不使用時緩慢回充
            mods.nitro_charge = (mods.nitro_charge + NITRO_RECHARGE_RATE * dt).min(1.0);
        }
    }
}

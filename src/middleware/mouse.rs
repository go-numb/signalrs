use mouse_rs::{types::keys::Keys, Mouse as MouseRs};
use rand::Rng;

use crate::invoke;

#[derive(Default)]
pub struct Mouse {}

#[allow(unused)]
pub enum MouseKeys {
    Left,
    Right,
}

impl MouseKeys {
    pub fn to_keys(&self) -> Keys {
        match self {
            MouseKeys::Left => Keys::LEFT,
            MouseKeys::Right => Keys::RIGHT,
        }
    }
}

impl Mouse {
    pub fn random_xy(&self, min_x: i32, min_y: i32, max_x: i32, max_y: i32) -> (i32, i32) {
        let mut rng = rand::thread_rng();

        let x = rng.gen_range(min_x..max_x);
        let y = rng.gen_range(min_y..max_y);
        (x, y)
    }

    #[allow(unused)]
    pub fn pos(&self) -> (i32, i32) {
        let rs = MouseRs::new();
        let pos = rs.get_position().unwrap();
        (pos.x, pos.y)
    }

    pub fn move_to(&self, x: i32, y: i32) {
        let rs = MouseRs::new();
        rs.move_to(x, y).unwrap()
    }

    #[allow(unused)]
    pub fn press(&self, key: &MouseKeys) {
        let rs = MouseRs::new();
        rs.press(&key.to_keys()).unwrap();
    }

    #[allow(unused)]
    pub fn release(&self, key: &MouseKeys) {
        let rs = MouseRs::new();
        rs.release(&key.to_keys()).unwrap();
    }

    #[allow(unused)]
    /// 都度mouseRs clientを取得するようにした
    /// Arc<Mutexなどで持ち運び、clone,lockするよりも都度取得した方が安全かつ完結
    /// マウスクライアントを持ちながら待機すると、マウスクライアントが解放されないため
    pub fn click(&self) {
        let rs = MouseRs::new();
        rs.click(&Keys::LEFT).unwrap();
    }

    pub fn order(&self, setting: &invoke::gui::Mouse) {
        let (min_x, min_y, max_x, max_y) = {
            (
                setting.start_x as i32,
                setting.start_y as i32,
                setting.end_x as i32,
                setting.end_y as i32,
            )
        };
        let (x, y) = self.random_xy(min_x, min_y, max_x, max_y);

        let rs = MouseRs::new();
        rs.move_to(x, y).unwrap();
        rs.click(&Keys::LEFT).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_xy() {
        let interval = 10;
        let min_ms = (interval as f32 * 0.5 * 1000f32) as u32;
        let max_ms = (interval as f32 * 1.5 * 1000f32) as u32;
        let mut rng = rand::thread_rng();

        let count = 10000;
        let mut numbers = Vec::new();
        for _ in 0..count {
            let number = rng.gen_range(min_ms..max_ms);
            numbers.push(number);
        }

        // 平均を求める
        let sum = numbers.iter().sum::<u32>();
        let average = sum as f32 / count as f32;
        println!("average: {}", average);
        println!("min: {}, max: {}", min_ms, max_ms);
    }
}

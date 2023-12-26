use std::collections::HashMap;

pub struct BinLattice {
    pub factor: i32,
    pub bins: HashMap<(i32, i32), Vec<u32>>,
}

impl BinLattice {
    pub fn new(factor: i32) -> BinLattice {
        BinLattice {
            factor,
            bins: HashMap::new(),
        }
    }

    pub fn key(&self, x: f32, y: f32) -> (i32, i32) {
        (x as i32 / self.factor, y as i32 / self.factor)
    }

    pub fn remove(&mut self, x: f32, y: f32, object_id: u32) {
        let key = self.key(x, y);

        if let Some(bin) = self.bins.get_mut(&key) {
            if let Some(index) = bin.iter().position(|oid| *oid == object_id) {
                bin.swap_remove(index);
            }
        }
    }

    pub fn put(&mut self, x: f32, y: f32, object_id: u32) {
        let key = self.key(x, y);

        if let Some(bin) = self.bins.get_mut(&key) {
            bin.push(object_id);
        } else {
            let mut b: Vec<u32> = Vec::new();
            b.push(object_id);
            self.bins.insert(key, b);
        }
    }

    pub fn get_nearby(&self, x: f32, y: f32, range: f32) -> Vec<u32> {
        let min_x = (x - range / 2.0) as i32 / self.factor;
        let max_x = (x + range / 2.0) as i32 / self.factor;
        let min_y = (y - range / 2.0) as i32 / self.factor;
        let max_y = (y + range / 2.0) as i32 / self.factor;

        let mut rv: Vec<u32> = Vec::new();
        for i in min_x..max_x {
            for j in min_y..max_y {
                let key = (i, j);
                if let Some(bin) = self.bins.get(&key) {
                    rv.append(&mut bin.clone());
                }
            }
        }

        // log::debug!("scan from {},{} -> {},{} to {},{} found {}", x, y, min_x, min_y, max_x, max_y, rv.len());

        rv
    }
}

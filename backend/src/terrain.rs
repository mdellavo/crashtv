extern crate noise;

use std::path::Path;
use std::thread;
use std::time::Instant;

use rand::Rng;

use noise::utils::{NoiseMap, NoiseMapBuilder, PlaneMapBuilder};
use noise::{Fbm, Perlin};

use image::{ImageBuffer, Rgb, RgbImage};

#[derive(Clone, Debug, Serialize)]
pub enum TerrainType {
    Bare = 0x0,
    Beach = 0x1,
    Grassland = 0x2,
    Ocean = 0x3,
    Scorched = 0x4,
    Shrubland = 0x5,
    Snow = 0x6,
    SubtropicalDesert = 0x7,
    Taiga = 0x8,
    TemperateDeciduousForest = 0x9,
    TemperateDesert = 0xa,
    TemperateRainForest = 0xb,
    TropicalRainForest = 0xc,
    TropicalSeasonalForest = 0xd,
    Tundra = 0xe,
}

#[derive(Clone, Debug, Serialize)]
pub struct Terrain {
    pub size: u32,
    pub elevation_map: Vec<f32>,
    pub terrain_map: Vec<u8>,
}

impl Terrain {
    pub fn new(size: u32) -> Terrain {
        let map = vec![vec![0.0; size as usize]; size as usize];

        println!("Generating terrain...");

        let t_generate = Instant::now();
        let elevation_join_handle: thread::JoinHandle<NoiseMap> = thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let elevation_fbm = Fbm::<Perlin>::new(rng.gen());
            PlaneMapBuilder::<Fbm<Perlin>, 2>::new(elevation_fbm)
                .set_size(size as usize, size as usize)
                .set_is_seamless(true)
                .set_x_bounds(-5.0, 5.0)
                .set_y_bounds(-5.0, 5.0)
                .build()
        });

        let moisture_join_handle: thread::JoinHandle<NoiseMap> = thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let moisture_fbm = Fbm::<Perlin>::new(rng.gen());
            PlaneMapBuilder::<Fbm<Perlin>, 2>::new(moisture_fbm)
                .set_size(size as usize, size as usize)
                .set_is_seamless(true)
                .set_x_bounds(-5.0, 5.0)
                .set_y_bounds(-5.0, 5.0)
                .build()
        });

        let elevation_map = elevation_join_handle.join().unwrap();
        let moisture_map = moisture_join_handle.join().unwrap();
        println!("done - {:?}", t_generate.elapsed());

        fn calc_biome(elevation: f32, moisture: f32) -> TerrainType {
            if elevation < 0.1 {
                return TerrainType::Ocean;
            } else if elevation < 0.12 {
                return TerrainType::Beach;
            } else if elevation > 0.8 {
                if moisture < 0.1 {
                    return TerrainType::Scorched;
                } else if moisture < 0.2 {
                    return TerrainType::Bare;
                } else if moisture < 0.5 {
                    return TerrainType::Tundra;
                } else {
                    return TerrainType::Snow;
                }
            } else if elevation > 0.6 {
                if moisture < 0.33 {
                    return TerrainType::TemperateDesert;
                } else if moisture < 0.66 {
                    return TerrainType::Shrubland;
                } else {
                    return TerrainType::Taiga;
                }
            } else if elevation > 0.3 {
                if moisture < 0.16 {
                    return TerrainType::TemperateDesert;
                } else if moisture < 0.6 {
                    return TerrainType::Grassland;
                } else if moisture < 0.83 {
                    return TerrainType::TemperateDeciduousForest;
                } else {
                    return TerrainType::TemperateRainForest;
                }
            } else {
                if moisture < 0.16 {
                    return TerrainType::SubtropicalDesert;
                } else if moisture < 0.33 {
                    return TerrainType::Grassland;
                } else if moisture < 0.66 {
                    return TerrainType::TropicalSeasonalForest;
                } else {
                    return TerrainType::TropicalRainForest;
                }
            }
        }

        fn get_terrain_color(terrain_type: &TerrainType) -> Rgb<u8> {
            match terrain_type {
                TerrainType::Ocean => Rgb([0, 153, 152]),
                TerrainType::Beach => Rgb([153, 255, 255]),
                TerrainType::Bare => Rgb([128, 128, 128]),
                TerrainType::Grassland => Rgb([102, 255, 102]),
                TerrainType::Scorched => Rgb([192, 192, 192]),
                TerrainType::Shrubland => Rgb([204, 204, 0]),
                TerrainType::Snow => Rgb([255, 255, 255]),
                TerrainType::SubtropicalDesert => Rgb([255, 204, 153]),
                TerrainType::Taiga => Rgb([0, 204, 102]),
                TerrainType::TemperateDeciduousForest => Rgb([102, 204, 0]),
                TerrainType::TemperateDesert => Rgb([255, 153, 51]),
                TerrainType::TemperateRainForest => Rgb([0, 204, 0]),
                TerrainType::TropicalRainForest => Rgb([0, 255, 0]),
                TerrainType::TropicalSeasonalForest => Rgb([51, 255, 51]),
                TerrainType::Tundra => Rgb([204, 229, 255]),
            }
        }

        println!("Rendering images...");
        let t_render = Instant::now();

        let mut terrain_image: RgbImage = ImageBuffer::new(size, size);
        let mut elevation_image: RgbImage = ImageBuffer::new(size, size);
        let mut moisture_image: RgbImage = ImageBuffer::new(size, size);

        fn get_map_value(map: &NoiseMap, x: u32, y: u32) -> f32 {
            (map.get_value(x as usize, y as usize) * 0.5 + 0.5)
                .clamp(0.0, 1.0)
                .powf(2.0) as f32
        }

        let mut terrain = Terrain {
            size,
            elevation_map: vec![],
            terrain_map: vec![],
        };

        for x in 0..size {
            for y in 0..size {
                let elevation = get_map_value(&elevation_map, x, y);
                terrain.elevation_map.push(elevation);

                let moisture = get_map_value(&moisture_map, x, y);

                let biome = calc_biome(elevation, moisture);
                terrain.terrain_map.push(biome.clone() as u8);

                let terrain_pixel = terrain_image.get_pixel_mut(x, y);
                *terrain_pixel = get_terrain_color(&biome);

                let elevation_pixel = elevation_image.get_pixel_mut(x, y);
                let elevation_val = (elevation * 255.0).round() as u8;
                *elevation_pixel = Rgb([elevation_val, elevation_val, elevation_val]);

                let moisture_pixel = moisture_image.get_pixel_mut(x, y);
                let moisture_val = (moisture * 255.0).round() as u8;
                *moisture_pixel = Rgb([moisture_val, moisture_val, moisture_val]);
            }
        }
        println!("done - {:?}", t_render.elapsed());

        let t_write = Instant::now();
        println!("Saving images...");
        let terrain_handle = thread::spawn(move || {
            terrain_image.save(Path::new("/tmp/terrain.png")).unwrap();
        });
        let elevation_handle = thread::spawn(move || {
            elevation_image
                .save(Path::new("/tmp/elevation.png"))
                .unwrap();
        });
        let moisture_handle = thread::spawn(move || {
            moisture_image.save(Path::new("/tmp/moisture.png")).unwrap();
        });

        terrain_handle.join().unwrap();
        elevation_handle.join().unwrap();
        moisture_handle.join().unwrap();

        println!("done - {:?}", t_write.elapsed());

        terrain
    }

    pub fn get_elevation(&self, x: u32, y: u32) -> f32 {
        let x = x.clamp(0, self.size - 1);
        let y = y.clamp(0, self.size - 1);
        let idx = (self.size * y) + x;
        self.elevation_map[idx as usize]
    }
}

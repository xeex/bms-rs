mod bms;

use bms::{format::BMS, parser::BmsParser, Alphanumeric};
use quicksilver::{
    geom::Vector,
    lifecycle::{run, Asset, Settings, State, Window},
    sound::Sound,
    Result,
};
use std::{collections::HashMap, env, fs::File, path::Path};

struct Scene {
    bms: BMS,
    loaded_keysounds: HashMap<Alphanumeric, Asset<Sound>>,
    elapsed: f64,
    last_played: usize,
}

impl State for Scene {
    fn new() -> Result<Scene> {
        let args: Vec<String> = env::args().collect();
        let bms_filename = &args[1];
        let mut f = File::open(bms_filename)?;
        let bp = BmsParser;
        let bms = bp.parse(&mut f);

        let mut loaded_keysounds: HashMap<Alphanumeric, Asset<Sound>> = HashMap::new();
        for (key, path) in bms.keysounds.iter() {
            loaded_keysounds.insert(
                key.clone(),
                Asset::new(Sound::load(
                    Path::new(bms_filename)
                        .parent()
                        .unwrap()
                        .join(path.to_owned()),
                )),
            );
        }

        Ok(Scene {
            bms,
            loaded_keysounds,
            elapsed: 0.,
            last_played: 0,
        })
    }

    fn update(&mut self, window: &mut Window) -> Result<()> {
        self.elapsed += window.update_rate();
        for iter in self.last_played..self.bms.objects.len() {
            let obj = &self.bms.objects[iter];
            match obj.objtype {
                bms::ObjType::Auto(an) | bms::ObjType::Note(an) => {
                    if obj.time as f64 <= self.elapsed {
                        self.loaded_keysounds
                            .get_mut(&an)
                            .expect("Keysound not found")
                            .execute(|sound| sound.play())
                            .unwrap();
                        self.last_played += 1;
                    } else {
                        break;
                    }
                }
                _ => {
                    self.last_played += 1;
                    continue;
                }
            }
        }
        Ok(())
    }
}

fn main() {
    run::<Scene>("BMS-rs", Vector::new(800, 600), Settings::default());
}

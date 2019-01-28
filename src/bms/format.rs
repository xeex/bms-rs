use crate::bms::{
    timeline::{Timeline, TimelineBuilder},
    Alphanumeric, Object,
};
/// A module for a data structure corresponding to the BMS format, as well as the parser.
///
use std::{collections::HashMap, path::PathBuf, vec::Vec};

#[derive(Debug)]
pub struct BMS {
    /// BMS file metadata.
    pub title: String,
    pub artist: String,
    pub metadata: HashMap<String, String>,
    pub objects: Vec<Object>,

    // Sound/timeline related fields.
    pub timeline: Timeline,
    pub keysounds: HashMap<Alphanumeric, String>,

    // BGA
    pub bga_layers: HashMap<Alphanumeric, String>,
}

pub struct BmsBuilder {
    pub metadata: HashMap<String, String>,
    pub objects: Vec<Object>,
    pub keysounds: HashMap<Alphanumeric, String>,
    pub bga_layers: HashMap<Alphanumeric, String>,
    pub timeline_builder: TimelineBuilder,
}

impl BmsBuilder {
    pub fn new() -> BmsBuilder {
        BmsBuilder {
            metadata: HashMap::new(),
            objects: Vec::new(),
            keysounds: HashMap::new(),
            bga_layers: HashMap::new(),
            timeline_builder: TimelineBuilder::new(),
        }
    }

    // Maybe it's better to consume the header and value
    pub fn with_metadata(&mut self, header: String, value: String) -> &Self {
        // self.metadata.insert(header.to_string(), value.to_string());
        self.metadata.insert(header, value);
        self
    }

    // Maybe it's better to consume the path
    pub fn with_keysound(&mut self, keysound_key: Alphanumeric, path: String) -> &Self {
        self.keysounds.insert(keysound_key, path);
        self
    }

    // Maybe it's better to consume the path
    pub fn with_bga_layer(&mut self, bga_key: Alphanumeric, path: String) -> &Self {
        self.bga_layers.insert(bga_key, path);
        self
    }

    pub fn with_bpm(&mut self, bpm_key: Alphanumeric, bpm: f32) -> &Self {
        self.timeline_builder.with_bpm(bpm_key, bpm);
        self
    }

    pub fn with_stop(&mut self, stop_key: Alphanumeric, stop: f32) -> &Self {
        self.timeline_builder.with_stop(stop_key, stop);
        self
    }

    pub fn add_object(&mut self, object: Object) -> &Self {
        self.objects.push(object);
        self
    }

    pub fn build(mut self) -> BMS {
        let title = dbg!(self
            .metadata
            .get("TITLE")
            .unwrap_or(&"MISSING TITLE".to_string())
            .to_string());
        let artist = dbg!(self
            .metadata
            .get("ARTIST")
            .unwrap_or(&"MISSING ARTIST".to_string())
            .to_string());

        // Sort objects by measure
        self.objects
            .sort_by(|o1, o2| o1.measure.partial_cmp(&o2.measure).unwrap());

        // Pre-build the timeline, so the object positions can be cached
        let timeline = self.timeline_builder.build();
        for mut object in self.objects.iter_mut() {
            object.time = timeline.time_from_measure(object.measure);
        }

        BMS {
            title,
            artist,
            metadata: self.metadata,
            objects: self.objects,
            timeline,
            keysounds: self.keysounds,
            bga_layers: self.bga_layers,
        }
    }
}

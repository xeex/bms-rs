/// A time-based representation of all timing events, such as BPM changes, STOPs, and lengths.
/// Each event will be recorded along with its measure and time, then concatenated together to form
/// a "timeline".
///
/// The idea is that if we can find the event under which the current time falls, then we can
/// guarantee that the measure progress rate will be constant, so starting with that event's
/// measure, we can calculate the remaining measure easily.
///
/// In the timeline, we will store Events that successively differ in either BPM or length.
/// STOP commands will be implemented by having two Events with the same BPM/length/measure
/// but different times.
///
/// For example, if we have a STOP command at measure 5.00 that runs for 20 seconds, then our
/// timeline would contain
/// `Event { time: 1000, measure: 5.00, length: 1.0  }`
/// `Event { time: 1020, measure: 5.00, length: 1.0  }`
/// Detection of STOP commands should be easy if we calculate the measure offset as
/// deltaMeasure/deltaTime.
use crate::bms::Alphanumeric;
use std::vec::Vec;

/// An event in the timeline. For every event that appears in the timeline, a change in the
/// length, BPM, and/or STOP occurs.
#[derive(Debug)]
pub struct Event {
    /// The time in milliseconds that the event appears.
    pub time: i64,
    /// The measure number in which the event appears
    pub measure: f32,
    /// The graphical y position in which the event appears, if the event were an object.
    pub pos: f32,
    /// The bpm of the event.
    pub bpm: f32,
    /// The length of a measure in the current event.
    pub length: f32,
}

/// The timeline struct, which contains Events in chronological order.
#[derive(Debug, Default)]
pub struct Timeline {
    pub events: Vec<Event>,
}

impl Timeline {
    /// Return last event in its raw Event struct form.
    pub fn last_event(&self) -> &Event {
        // Since Timelines will only be generated from the TimelineBuilder, there must always be
        // at least one event at all times.
        self.events.last().expect("Empty timeline")
    }

    /// Add a new event if provided as measure and length
    pub fn add_event(&mut self, measure: f32, bpm: f32, length: f32) {
        // If there are no events, just add the current event
        if self.events.is_empty() {
            self.events.push(Event {
                time: 0i64,
                measure,
                pos: 0.0,
                bpm,
                length,
            });
        } else {
            // Only add if the event is an actual update (new BPM or new length)
            let &Event {
                mut time,
                measure: old_measure,
                bpm: old_bpm,
                length: old_length,
                ..
            } = self.last_event();
            if old_bpm != bpm || old_length != length {
                // Calculate new time
                time += ((measure - old_measure) * (240_000f32 / old_bpm) * old_length) as i64;
                self.events.push(Event {
                    time,
                    measure,
                    pos: 0.0,
                    bpm,
                    length,
                });
            }
        }
    }

    /// Add a new stop event; only time changes between this event and the last one
    /// The `stop_arg` parameter is the channel 02 argument (after its alphanumeric mapping has
    /// been resolved).
    pub fn add_stop_event(&mut self, measure: f32, stop_arg: f32) {
        let &Event {
            mut time,
            measure: old_measure,
            pos,
            bpm,
            length,
        } = self.last_event();
        if old_measure != measure {
            // Add another event that 'snapshots' the current BPM and time, so we can show how long
            // the measure stays stopped for.
            // We can't use `self.add_event`, since there are no BPM nor length changes.
            time += ((measure - old_measure) * (240_000f32 / bpm) * length) as i64;
            self.events.push(Event {
                time,
                measure,
                pos,
                bpm,
                length,
            });
        }
        // Now we can add the STOP event, which has the same measure, bpm, and length as the last
        // event, but has additional time that is proportional to the `stop_arg`.
        time += (240_000f32 / bpm * stop_arg / 192f32) as i64;
        self.events.push(Event {
            time,
            measure,
            pos,
            bpm,
            length,
        });
    }

    /// Convert a measure value to a time position
    pub fn time_from_measure(&self, measure: f32) -> i64 {
        // Sortedness of events is guaranteed, so we first find the segment `measure` falls under
        let event_index = Timeline::last_event_index_in_measure(&self.events, measure);
        // The time increase within this event is proportional to time
        let Event {
            time: curr_time,
            measure: curr_measure,
            bpm: curr_bpm,
            length: curr_length,
            ..
        } = self.events[event_index];
        let remaining_measure = measure - curr_measure;
        let remaining_time = (remaining_measure * (240_000f32 / curr_bpm) * curr_length) as i64;

        curr_time + remaining_time
    }

    /// Cache the position of each event
    pub fn cache_event_pos(&mut self) {
        for i in 1..self.events.len() {
            self.events[i].pos = self.events[i - 1].pos
                + ((self.events[i].measure - self.events[i - 1].measure)
                    * self.events[i - 1].bpm
                    * self.events[i - 1].length);
        }
    }

    /// Convert a measure to render position
    pub fn pos_from_measure(&self, measure: f32, speed: f32) -> f32 {
        // Find the event block under which this measure is in
        let event_index = Timeline::last_event_index_in_measure(&self.events, measure);
        let event_block = &self.events[event_index];
        let mut pos = event_block.pos * speed;
        // Adding the last little bit of position
        pos += ((measure - event_block.measure) * speed * event_block.bpm) * event_block.length;
        pos
    }

    /// Convert a time position to a measure value
    pub fn measure_from_time(&self, time: i64) -> f32 {
        // If we can guarantee sortedness of events, this should be fine
        // First, find the event segment `time` falls under
        let event_index = Timeline::last_event_index_in_time(&self.events, time);

        // Check if the time falls within a STOP command
        if event_index < self.events.len() - 1
            && self.events[event_index].measure == self.events[event_index + 1].measure
        {
            // Just return the current measure
            self.events[event_index].measure
        } else {
            // The measure within this event is proportional to time
            let Event {
                time: curr_time,
                measure: curr_measure,
                bpm: curr_bpm,
                length: curr_length,
                ..
            } = self.events[event_index];
            let remaining_time = time - curr_time;
            let remaining_measure: f32 =
                remaining_time as f32 * (curr_bpm / 240_000f32) / curr_length;

            curr_measure + remaining_measure
        }
    }

    /// Find the index of the last event in `v` before `measure`
    fn last_event_index_in_measure(v: &[Event], measure: f32) -> usize {
        match v.binary_search_by(|e| e.measure.partial_cmp(&measure).unwrap()) {
            Ok(mut i) => {
                /* measure found in v at index i */
                // Continue iteration until we hit the last event
                // This occurs during STOP commands
                while i < v.len() - 1 && v[i + 1].measure == measure {
                    i += 1;
                }
                i
            }
            Err(i) => {
                /* measure not found in v, but should belong in index i */
                // Since we want the last event to affect `measure`, return i-1.
                // In the case where `i == 0`, only the first event affects the measure, so the
                // function returns 0.
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
        }
    }

    /// Find the index of the last event in `v` before `time`
    fn last_event_index_in_time(v: &[Event], time: i64) -> usize {
        match v.binary_search_by(|e| e.time.partial_cmp(&time).unwrap()) {
            Ok(mut i) => {
                while i < v.len() - 1 && v[i + 1].time == time {
                    i += 1;
                }
                i
            }
            Err(i) => {
                if i == 0 {
                    0
                } else {
                    i - 1
                }
            }
        }
    }
}

use std::collections::HashMap;

pub enum TimelineEvent {
    BPM { measure: f32, bpm: f32 },
    STOP { measure: f32, duration: f32 },
}

pub struct TimelineBuilder {
    // Data for building the timeline
    base_bpm: f32,                     // #BPM XX
    bpms: HashMap<Alphanumeric, f32>,  // BPM mapping (only applies to channel 8)
    stops: HashMap<Alphanumeric, f32>, // STOP mapping
    measure_lens: Vec<f32>,            // Measure lengths
    events: Vec<TimelineEvent>,        // Collection of all events in the timeline
}

impl TimelineBuilder {
    pub fn new() -> Self {
        TimelineBuilder {
            base_bpm: 130_f32, // 130 BPM is the default according to the specifications
            bpms: HashMap::new(),
            stops: HashMap::new(),
            measure_lens: vec![1.0; 1000],
            events: Vec::new(),
        }
    }

    pub fn build(&mut self) -> Timeline {
        // Creating the timeline from all gathered timing data.

        // Find all measures where timeline events occur.
        let mut timeline_measures: Vec<f32> = Vec::new();
        // We'll also separate the BPM events from the STOP events, which will make building easier
        let mut bpm_measures: Vec<(f32, f32)> = Vec::new();
        let mut stop_measures: Vec<(f32, f32)> = Vec::new();

        for event in &self.events {
            match event {
                TimelineEvent::BPM { measure: m, bpm: b } => {
                    bpm_measures.push((*m, *b));
                    timeline_measures.push(*m);
                }
                TimelineEvent::STOP {
                    measure: m,
                    duration: d,
                } => {
                    stop_measures.push((*m, *d));
                    timeline_measures.push(*m);
                }
            }
        }

        // Find indices of change in `measure_lens` and append them to `timeline_measures`
        let measure_indices = {
            let mut out: Vec<(u16, f32)> = Vec::new();
            let mut last_value = *self.measure_lens.first().unwrap();
            out.push((0, last_value));
            for (i, &elem) in self.measure_lens.iter().enumerate().skip(1) {
                if last_value != elem {
                    out.push((i as u16, elem));
                    last_value = elem;
                }
            }
            out
        };
        for (measure, _length) in &measure_indices {
            timeline_measures.push(*measure as f32);
        }

        // Remove duplicate measures
        timeline_measures.sort_by(|a, b| a.partial_cmp(b).unwrap());
        timeline_measures.dedup();
        // Sort BPM and STOP measures as well
        bpm_measures.sort_by(|(a1, _a2), (b1, _b2)| a1.partial_cmp(b1).unwrap());
        stop_measures.sort_by(|(a1, _a2), (b1, _b2)| a1.partial_cmp(b1).unwrap());

        // Begin creating a Timeline of Events
        let mut timeline = Timeline { events: Vec::new() };
        // Memoizing the last event to occur in each category
        // last_bpm is equal to the base BPM if there are no BPM changes, or the first BPM change does not occur at measure 0.
        let mut last_bpm: f32 = if bpm_measures.is_empty()
            || (bpm_measures[0].0 - 0_f32) > 0.000_001
        /* Not equal */
        {
            self.base_bpm
        } else {
            bpm_measures[0].1
        };
        let mut last_len: f32 = measure_indices[0].1;
        // Add the first event in timeline
        // timeline.add_event(0_f32, last_bpm, last_len);

        // Timeline construction
        // Update the memoized values whenever the BPM or length changes
        for measure in timeline_measures {
            // Update BPM if applicable
            if let Ok(index) =
                bpm_measures.binary_search_by(|(a, _b)| a.partial_cmp(&measure).unwrap())
            {
                last_bpm = bpm_measures[index].1;
            }
            // Update measure length if applicable
            if (measure.floor() - measure).abs() <= ::std::f32::EPSILON {
                if let Ok(index) =
                    measure_indices.binary_search_by(|(a, _b)| a.cmp(&(measure.floor() as u16)))
                {
                    last_len = measure_indices[index].1;
                }
            }
            // Add stop if applicable
            if let Ok(index) =
                stop_measures.binary_search_by(|(a, _b)| a.partial_cmp(&measure).unwrap())
            {
                timeline.add_stop_event(measure, stop_measures[index].1);
            }
            timeline.add_event(measure, last_bpm, last_len);
        }

        // Finally, cache the positions of events
        timeline.cache_event_pos();

        timeline
    }

    /// Sets the base bpm. The base bpm will always be the first event, unless another event with a different bpm has been inserted using <em>with_event</em>
    pub fn with_base_bpm(&mut self, base_bpm: f32) -> &Self {
        self.base_bpm = base_bpm;
        self
    }

    pub fn with_bpm(&mut self, bpm_key: Alphanumeric, bpm: f32) -> &Self {
        self.bpms.insert(bpm_key, bpm);
        self
    }

    pub fn with_stop(&mut self, stop_key: Alphanumeric, stop: f32) -> &Self {
        self.stops.insert(stop_key, stop);
        self
    }

    /// Adds a timeline event to the builder.
    pub fn with_event(&mut self, event: TimelineEvent) -> &Self {
        self.events.push(event);
        self
    }

    pub fn with_measure_len(&mut self, measure: u32, length: f32) -> &Self {
        self.measure_lens[measure as usize] = length;
        self
    }

    /// Finds a previously-inserted bpm value with the given key
    pub fn find_bpm(&self, bpm_key: Alphanumeric) -> f32 {
        *self.bpms.get(&bpm_key).unwrap_or(&0_f32)
    }

    /// Finds a previously-inserted stop value with the given key
    pub fn find_stop(&self, stop_key: Alphanumeric) -> f32 {
        *self.stops.get(&stop_key).unwrap_or(&0_f32)
    }
}

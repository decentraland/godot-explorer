use std::collections::HashMap;

use godot::prelude::Dictionary;

pub struct DictionaryToHashMap;

impl DictionaryToHashMap {
    pub fn convert(dictionary: Dictionary) -> HashMap<String, String> {
        let mut hashmap = HashMap::with_capacity(dictionary.len());
        for (key, value) in dictionary.iter_shared() {
            hashmap.insert(key.to_string(), value.to_string());
        }
        hashmap
    }
}

// type Parcel = (i16, i16);

// const INNER_SIZE: i16 = 4;
// const OUTER_SIZE: i16 = 5;

// pub struct ParcelLoader {
//     current_position: Parcel,
//     parcels: Vec<Parcel>,
// }

// impl ParcelLoader {
//     pub fn new(current_position: Parcel) -> ParcelLoader {
//         ParcelLoader {
//             current_position,
//             parcels: vec![],
//         }
//     }

//     pub fn set_new_position(&mut self, new_parcels: Vec<Parcel>) {
//         let (cx, cy) = self.current_position;
//         for x in (cx - OUTER_SIZE)..=(cx + OUTER_SIZE) {
//             for y in (cy - OUTER_SIZE)..=(cy + OUTER_SIZE) {
//                 let distance = Self::distance(&self.current_position, &(x, y));

//                 if self.distance(&(x, y), &self.current_position) <= INNER_SIZE {
//                     self.parcels.push((x, y));
//                 }
//             }
//         }

//         for parcel in new_parcels {
//             // Check if the parcel is within the inner size
//             if self.distance(&parcel, &self.current_position) <= self.inner_size {
//                 self.parcels.push(parcel);
//             }
//         }
//     }

//     pub fn get_parcels(&self) -> &Vec<Parcel> {
//         &self.parcels
//     }

//     fn distance(a: &Parcel, b: &Parcel) -> i16 {
//         let (x1, y1) = a;
//         let (x2, y2) = b;

//         // Using Euclidean distance
//         (((*x2 as f64 - *x1 as f64).powi(2) + (*y2 as f64 - *y1 as f64).powi(2)).sqrt() as i16)
//             .abs()
//     }
// }

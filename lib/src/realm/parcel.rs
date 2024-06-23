use std::collections::HashSet;

use godot::builtin::Vector2i;

#[derive(Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Coord(pub i16, pub i16);

#[derive(Debug)]
pub struct ParcelRadiusCalculator {
    outter_parcels: HashSet<Coord>,
    inner_parcels: HashSet<Coord>,
}

impl From<&String> for Coord {
    fn from(s: &String) -> Self {
        let mut iter = s.split(',');

        let x: i16 = match iter.next() {
            Some(value) => value.parse().unwrap_or(0),
            None => 0,
        };

        let y: i16 = match iter.next() {
            Some(value) => value.parse().unwrap_or(0),
            None => 0,
        };

        Coord(x, y)
    }
}

impl From<&Vector2i> for Coord {
    fn from(v: &Vector2i) -> Self {
        Coord(v.x as i16, v.y as i16)
    }
}

impl Coord {
    pub fn plus(&self, other: &Coord) -> Self {
        Coord(self.0 + other.0, self.1 + other.1)
    }
}

impl std::fmt::Display for Coord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{}", self.0, self.1)
    }
}

impl Default for ParcelRadiusCalculator {
    fn default() -> Self {
        Self::new(3)
    }
}

impl ParcelRadiusCalculator {
    pub fn new(parcel_radius: i16) -> Self {
        // Clamp
        let parcel_radius = parcel_radius.clamp(0, 5);

        let parcel_radius_squared = (parcel_radius * parcel_radius) as f32;
        let mut outter_parcels = HashSet::new();
        let mut inner_parcels = HashSet::new();
        for x in -parcel_radius..=parcel_radius {
            for z in -parcel_radius..=parcel_radius {
                let distance_squared = (x * x + z * z) as f32;
                if distance_squared > parcel_radius_squared {
                    continue;
                }
                inner_parcels.insert(Coord(x, z));
            }
        }

        let parcel_radius = parcel_radius + 2;
        let parcel_radius_squared = (parcel_radius * parcel_radius) as f32;
        for x in -parcel_radius - 1..=parcel_radius + 1 {
            for z in -parcel_radius - 1..=parcel_radius + 1 {
                let distance_squared = (x * x + z * z) as f32;
                if distance_squared > parcel_radius_squared {
                    continue;
                }
                if inner_parcels.contains(&Coord(x, z)) {
                    continue;
                }
                outter_parcels.insert(Coord(x, z));
            }
        }

        Self {
            outter_parcels,
            inner_parcels,
        }
    }

    pub fn get_inner_parcels(&self) -> &HashSet<Coord> {
        &self.inner_parcels
    }

    pub fn get_outer_parcels(&self) -> &HashSet<Coord> {
        &self.outter_parcels
    }
}

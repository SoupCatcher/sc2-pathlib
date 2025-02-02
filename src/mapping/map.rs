use crate::{helpers::round_point2, path_find::PathFind};
use pyo3::prelude::*;

// extern crate test;
use std::collections::HashSet;

use super::chokes::{group_chokes, Choke};
use crate::mapping::chokes::solve_chokes;
use crate::mapping::climb::modify_climb;
use crate::mapping::map_point;
use crate::mapping::map_point::Cliff;

const DIFFERENCE: usize = 16;
const Y_MULT: usize = 1000000;

/// Mapping for python-sc2
#[pyclass]
pub struct Map {
    pub ground_pathing: PathFind,
    pub air_pathing: PathFind,
    pub colossus_pathing: PathFind,
    pub reaper_pathing: PathFind,
    pub points: Vec<Vec<map_point::MapPoint>>,
    pub overlord_spots: Vec<(f32, f32)>,
    #[pyo3(get, set)]
    pub influence_colossus_map: bool,
    #[pyo3(get, set)]
    pub influence_reaper_map: bool,
    pub chokes: Vec<Choke>,
}

#[pymethods]
impl Map {
    #[new]
    fn new_py(pathing: Vec<Vec<usize>>,
              placement: Vec<Vec<usize>>,
              height_map: Vec<Vec<usize>>,
              x_start: usize,
              y_start: usize,
              x_end: usize,
              y_end: usize)
              -> Self {
        Map::new(pathing, placement, height_map, x_start, y_start, x_end, y_end)
    }

    #[getter(ground_pathing)]
    fn get_ground_pathing(&self) -> Vec<Vec<usize>> { self.ground_pathing.map.clone() }

    #[getter(air_pathing)]
    fn get_air_pathing(&self) -> Vec<Vec<usize>> { self.air_pathing.map.clone() }

    #[getter(reaper_pathing)]
    fn get_reaper_pathing(&self) -> Vec<Vec<usize>> { self.reaper_pathing.map.clone() }

    #[getter(colossus_pathing)]
    fn get_colossus_pathing(&self) -> Vec<Vec<usize>> { self.colossus_pathing.map.clone() }

    #[getter(overlord_spots)]
    fn get_overlord_spots(&self) -> Vec<(f32, f32)> { self.overlord_spots.clone() }

    #[getter(chokes)]
    pub fn get_chokes(&self) -> Vec<Choke> { self.chokes.clone() }

    fn draw_climbs(&self) -> Vec<Vec<usize>> {
        let width = self.ground_pathing.map.len();
        let height = self.ground_pathing.map[0].len();
        let mut walk_map = vec![vec![0; height]; width];
        let path = &self.ground_pathing.map;

        for x in 0..width {
            for y in 0..height {
                if path[x][y] > 0 {
                    if self.points[x][y].cliff_type == Cliff::High {
                        walk_map[x][y] = 5;
                    } else if self.points[x][y].cliff_type == Cliff::Both {
                        walk_map[x][y] = 4;
                    } else if self.points[x][y].cliff_type == Cliff::Low {
                        walk_map[x][y] = 3;
                    } else {
                        walk_map[x][y] = 2;
                    }
                } else if self.points[x][y].climbable {
                    walk_map[x][y] = 1;
                } else if self.points[x][y].overlord_spot {
                    walk_map[x][y] = 6;
                }
            }
        }

        walk_map
    }

    fn draw_chokes(&self) -> Vec<Vec<usize>> {
        let width = self.ground_pathing.map.len();
        let height = self.ground_pathing.map[0].len();
        let mut walk_map = vec![vec![0; height]; width];

        for x in 0..width {
            for y in 0..height {
                let point = &self.points[x][y];
                if point.is_border {
                    if point.is_choke {
                        walk_map[x][y] = 175;
                    } else {
                        walk_map[x][y] = 255;
                    }
                } else if point.is_choke {
                    walk_map[x][y] = 100;
                }
            }
        }

        walk_map
    }

    /// Reset all mapping to their originals.
    pub fn reset(&mut self) {
        self.ground_pathing.reset_void();
        self.air_pathing.reset_void();
        self.colossus_pathing.reset_void();
        self.reaper_pathing.reset_void();
    }

    pub fn create_block(&mut self, center: (f32, f32), size: (usize, usize)) {
        self.ground_pathing.create_block(center, size);
        self.colossus_pathing.create_block(center, size);
        self.reaper_pathing.create_block(center, size);
    }

    pub fn create_blocks(&mut self, centers: Vec<(f32, f32)>, size: (usize, usize)) {
        self.ground_pathing.create_blocks_rust(&centers, size);
        self.colossus_pathing.create_blocks_rust(&centers, size);
        self.reaper_pathing.create_blocks_rust(&centers, size);
    }

    pub fn remove_blocks(&mut self, centers: Vec<(f32, f32)>, size: (usize, usize)) {
        self.ground_pathing.remove_blocks_rust(&centers, size);
        self.colossus_pathing.remove_blocks_rust(&centers, size);
        self.reaper_pathing.remove_blocks_rust(&centers, size);
    }

    pub fn get_borders(&self) -> Vec<(usize, usize)> {
        let mut result = Vec::<(usize, usize)>::new();

        for x in 0..self.ground_pathing.width {
            for y in 0..self.ground_pathing.height {
                if self.points[x][y].is_border {
                    result.push((x, y));
                }
            }
        }

        result
    }

    /// Returns current influence value
    fn current_influence(&self, map_type: u8, position: (f32, f32)) -> f32 {
        let map = self.get_map(map_type);
        let position_int = round_point2(position);

        map.current_influence(position_int) as f32
    }

    /// Finds the first reachable position within specified walking distance from the center point with lowest value
    fn lowest_influence_walk(&self, map_type: u8, center: (f32, f32), distance: f32) -> ((usize, usize), f32) {
        let map = self.get_map(map_type);
        let center_int = round_point2(center);

        map.lowest_influence_walk(center_int, distance)
    }

    /// Finds the first reachable position within specified distance from the center point with lowest value
    pub fn lowest_influence(&self, map_type: u8, center: (f32, f32), distance: usize) -> ((usize, usize), f32) {
        let map = self.get_map(map_type);
        map.inline_lowest_value(center, distance)
    }

    /// Find the shortest path values without considering influence and returns the path and distance
    pub fn find_path(&self,
                     map_type: u8,
                     start: (f32, f32),
                     end: (f32, f32),
                     possible_heuristic: Option<u8>)
                     -> (Vec<(usize, usize)>, f32) {
        let start_int = (start.0.round() as usize, start.1.round() as usize);
        let end_int = (end.0.round() as usize, end.1.round() as usize);

        let map = self.get_map(map_type);
        map.find_path(start_int, end_int, possible_heuristic)
    }

    /// Find the shortest path values without considering influence and returns the path and distance
    pub fn find_path_large(&self,
                           map_type: u8,
                           start: (f32, f32),
                           end: (f32, f32),
                           possible_heuristic: Option<u8>)
                           -> (Vec<(usize, usize)>, f32) {
        let start_int = (start.0.round() as usize, start.1.round() as usize);
        let end_int = (end.0.round() as usize, end.1.round() as usize);

        let map = self.get_map(map_type);
        map.find_path_large(start_int, end_int, possible_heuristic)
    }

    /// Find the path using influence values and returns the path and distance
    pub fn find_path_influence(&self,
                               map_type: u8,
                               start: (f32, f32),
                               end: (f32, f32),
                               possible_heuristic: Option<u8>)
                               -> (Vec<(usize, usize)>, f32) {
        let start_int = (start.0.round() as usize, start.1.round() as usize);
        let end_int = (end.0.round() as usize, end.1.round() as usize);
        let map = self.get_map(map_type);
        map.find_path_influence(start_int, end_int, possible_heuristic)
    }

    /// Find the path using influence values and returns the path and distance
    pub fn find_path_influence_large(&self,
                                     map_type: u8,
                                     start: (f32, f32),
                                     end: (f32, f32),
                                     possible_heuristic: Option<u8>)
                                     -> (Vec<(usize, usize)>, f32) {
        let start_int = (start.0.round() as usize, start.1.round() as usize);
        let end_int = (end.0.round() as usize, end.1.round() as usize);
        let map = self.get_map(map_type);
        map.find_path_influence_large(start_int, end_int, possible_heuristic)
    }

    /// Find the shortest path values without considering influence and returns the path and distance
    pub fn find_path_closer_than(&self,
                                 map_type: u8,
                                 start: (f32, f32),
                                 end: (f32, f32),
                                 possible_heuristic: Option<u8>,
                                 distance_from_target: f32)
                                 -> (Vec<(usize, usize)>, f32) {
        let start_int = (start.0.round() as usize, start.1.round() as usize);
        let end_int = (end.0.round() as usize, end.1.round() as usize);

        let map = self.get_map(map_type);
        map.find_path_closer_than(start_int, end_int, possible_heuristic, distance_from_target)
    }

    /// Find the shortest path values without considering influence and returns the path and distance
    pub fn find_path_large_closer_than(&self,
                                       map_type: u8,
                                       start: (f32, f32),
                                       end: (f32, f32),
                                       possible_heuristic: Option<u8>,
                                       distance_from_target: f32)
                                       -> (Vec<(usize, usize)>, f32) {
        let start_int = (start.0.round() as usize, start.1.round() as usize);
        let end_int = (end.0.round() as usize, end.1.round() as usize);

        let map = self.get_map(map_type);
        map.find_path_large_closer_than(start_int, end_int, possible_heuristic, distance_from_target)
    }

    /// Find the path using influence values and returns the path and distance
    pub fn find_path_influence_closer_than(&self,
                                           map_type: u8,
                                           start: (f32, f32),
                                           end: (f32, f32),
                                           possible_heuristic: Option<u8>,
                                           distance_from_target: f32)
                                           -> (Vec<(usize, usize)>, f32) {
        let start_int = (start.0.round() as usize, start.1.round() as usize);
        let end_int = (end.0.round() as usize, end.1.round() as usize);
        let map = self.get_map(map_type);
        map.find_path_influence_closer_than(start_int, end_int, possible_heuristic, distance_from_target)
    }

    /// Find the path using influence values and returns the path and distance
    pub fn find_path_influence_large_closer_than(&self,
                                                 map_type: u8,
                                                 start: (f32, f32),
                                                 end: (f32, f32),
                                                 possible_heuristic: Option<u8>,
                                                 distance_from_target: f32)
                                                 -> (Vec<(usize, usize)>, f32) {
        let start_int = (start.0.round() as usize, start.1.round() as usize);
        let end_int = (end.0.round() as usize, end.1.round() as usize);
        let map = self.get_map(map_type);
        map.find_path_influence_large_closer_than(start_int, end_int, possible_heuristic, distance_from_target)
    }

    /// Finds a compromise where low influence matches with close position to the start position.
    fn find_low_inside_walk(&self,
                            map_type: u8,
                            start: (f32, f32),
                            target: (f32, f32),
                            distance: f32)
                            -> ((f32, f32), f32) {
        let map = self.get_map(map_type);
        map.find_low_inside_walk(start, target, distance)
    }
}

impl Map {
    pub fn new(pathing: Vec<Vec<usize>>,
               placement: Vec<Vec<usize>>,
               height_map: Vec<Vec<usize>>,
               x_start: usize,
               y_start: usize,
               x_end: usize,
               y_end: usize)
               -> Self {
        let width = pathing.len();
        let height = pathing[0].len();
        let mut points = vec![vec![map_point::MapPoint::new(); height]; width];

        let mut walk_map = vec![vec![0; height]; width];
        let mut border_map = vec![vec![0; height]; width];
        let mut fly_map = vec![vec![0; height]; width];
        let mut reaper_map = vec![vec![0; height]; width];
        let mut overlord_spots: Vec<(f32, f32)> = Vec::new();

        let mut choke_lines = Vec::<((usize, usize), (usize, usize))>::new();
        let x_left_border = x_start - 1;
        let y_top_border = y_start - 1;
        // Pass 1
        for x in 0..width {
            for y in 0..height {
                let walkable = pathing[x][y] > 0 || placement[x][y] > 0;
                let pathable = x_start <= x && x <= x_end && y_start <= y && y <= y_end;
                points[x][y].walkable = walkable;
                points[x][y].pathable = pathable;
                points[x][y].height = height_map[x][y];

                if pathable {
                    fly_map[x][y] = 1;
                }
                if walkable {
                    walk_map[x][y] = 1;
                    reaper_map[x][y] = 1;
                }

                if x == x_left_border || x == x_end || y == y_top_border || y == y_end {
                    border_map[x][y] = 1;
                }
            }
        }

        // Pass 2
        for x in x_start..x_end {
            for y in y_start..y_end {
                if !points[x][y].walkable {
                    let h0 = points[x][y + 1].height;
                    let h1 = points[x][y - 1].height;
                    if (points[x][y].height >= h0 + DIFFERENCE && h0 > 0)
                       || (points[x][y].height >= h1 + DIFFERENCE && h1 > 0)
                    {
                        points[x][y].overlord_spot = true;
                    }

                    if points[x + 1][y + 1].walkable
                       || points[x - 1][y + 1].walkable
                       || points[x + 1][y].walkable
                       || points[x - 1][y].walkable
                       || points[x + 1][y - 1].walkable
                       || points[x - 1][y - 1].walkable
                       || points[x][y + 1].walkable
                       || points[x][y - 1].walkable
                    {
                        points[x][y].is_border = true;
                        border_map[x][y] = 1;
                    }

                    continue;
                }

                modify_climb(&mut points, x as i32, y as i32, -1, -1);
                modify_climb(&mut points, x as i32, y as i32, 1, -1);
                modify_climb(&mut points, x as i32, y as i32, 1, 0);
                modify_climb(&mut points, x as i32, y as i32, 0, 1);
            }
        }

        // Required for pass 3 choke detection
        let ground_pathing = PathFind::new_internal(walk_map);
        let border_pathing = PathFind::new_internal(border_map);

        // Pass 3
        let mut set_handled_overlord_spots: HashSet<usize> = HashSet::new();
        for x in x_start..x_end {
            for y in y_start..y_end {
                let point_hash = x + y * Y_MULT;
                if points[x][y].climbable {
                    points[x][y].climbable = points[x + 1][y].climbable
                                             || points[x - 1][y].climbable
                                             || points[x][y + 1].climbable
                                             || points[x][y - 1].climbable;
                    if points[x][y].climbable {
                        reaper_map[x][y] = 1;
                    }
                }

                solve_chokes(&mut points, &border_pathing, &mut choke_lines, x, y, x_start, y_start, x_end, y_end);

                let c = points[x][y].cliff_type;

                if c != Cliff::None
                   && points[x + 1][y].cliff_type != c
                   && points[x - 1][y].cliff_type != c
                   && points[x][y + 1].cliff_type != c
                   && points[x][y - 1].cliff_type != c
                {
                    points[x][y].cliff_type = Cliff::None;
                }

                if !set_handled_overlord_spots.contains(&point_hash) && points[x][y].overlord_spot {
                    let target_height = points[x][y].height;
                    let mut set: HashSet<usize> = HashSet::new();

                    if flood_fill_overlord(&mut points, x, y, target_height, true, &mut set) {
                        let mut spot = (0_f32, 0_f32);
                        let count = set.len();
                        for value in set {
                            set_handled_overlord_spots.insert(value);
                            let cx = (value % Y_MULT) as f32;
                            let cy = (value / Y_MULT) as f32;
                            spot = (spot.0 + cx, spot.1 + cy);
                        }

                        spot = (spot.0 / count as f32, spot.1 / count as f32);
                        overlord_spots.push(spot);
                    } else {
                        set.clear();
                        flood_fill_overlord(&mut points, x, y, target_height, false, &mut set);
                    }
                }
            }
        }

        let air_pathing = PathFind::new_internal(fly_map);
        let colossus_pathing = PathFind::new_internal(reaper_map.clone());
        let reaper_pathing = PathFind::new_internal(reaper_map);

        let influence_colossus_map = false;
        let influence_reaper_map = false;
        let chokes = group_chokes(&mut choke_lines, &mut points);

        Map { ground_pathing,
              air_pathing,
              colossus_pathing,
              reaper_pathing,
              points,
              overlord_spots,
              influence_colossus_map,
              influence_reaper_map,
              chokes }
    }

    fn get_map(&self, map_type: u8) -> &PathFind {
        if map_type == 0 {
            return &self.ground_pathing;
        }
        if map_type == 1 {
            return &self.reaper_pathing;
        }
        if map_type == 2 {
            return &self.colossus_pathing;
        }
        if map_type == 3 {
            return &self.air_pathing;
        }

        panic!("Map type {} does not exist", map_type.to_string());
    }

    pub fn get_map_mut(&mut self, map_type: u8) -> &mut PathFind {
        if map_type == 0 {
            return &mut self.ground_pathing;
        }
        if map_type == 1 {
            return &mut self.reaper_pathing;
        }
        if map_type == 2 {
            return &mut self.colossus_pathing;
        }
        if map_type == 3 {
            return &mut self.air_pathing;
        }

        panic!("Map type {} does not exist", map_type.to_string());
    }
}

fn flood_fill_overlord(points: &mut Vec<Vec<map_point::MapPoint>>,
                       x: usize,
                       y: usize,
                       target_height: usize,
                       replacement: bool,
                       set: &mut HashSet<usize>)
                       -> bool {
    let key = x + y * Y_MULT;
    if set.contains(&key) {
        return true;
    }

    set.insert(key);

    if target_height != points[x][y].height {
        // Height difference must be at least 16 below target
        if target_height < points[x][y].height + DIFFERENCE {
            return false;
        }

        return true; // Could still be overlord spot.
    }

    let mut result = true;
    points[x][y].overlord_spot = replacement;

    if y > 0 {
        result &= flood_fill_overlord(points, x, ((y as u32) - 1) as usize, target_height, replacement, set);
    }
    if x > 0 {
        result &= flood_fill_overlord(points, ((x as u32) - 1) as usize, y, target_height, replacement, set);
    }
    if y < points[0].len() - 1 {
        result &= flood_fill_overlord(points, x, y + 1, target_height, replacement, set);
    }
    if x < points.len() - 1 {
        result &= flood_fill_overlord(points, x + 1, y, target_height, replacement, set);
    }

    result
}

use crate::error::{DwExtractError, Result};
use crate::formats::tim::TimHeader;
use std::io::{Read, Seek, SeekFrom};

fn read_i16<R: Read>(reader: &mut R) -> Result<i16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(i16::from_le_bytes(buf))
}

fn read_i32<R: Read>(reader: &mut R) -> Result<i32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

fn read_u32<R: Read>(reader: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_usize<R: Read>(reader: &mut R) -> Result<usize> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf) as usize)
}

pub const COLLISION_SIZE: usize = 100;
pub const MAX_TILES: usize = 35;
pub const MAX_INSTANCES: usize = 188;
pub const MAX_NPCS: usize = 8;
pub const MAX_WAYPOINTS: usize = 8;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MapLight {
    pub vx: i32,
    pub vy: i32,
    pub vz: i32,
    pub r: i32,
    pub g: i32,
    pub b: i32,
}

impl MapLight {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0u8; 24];
        reader.read_exact(&mut buf)?;
        Ok(Self {
            vx: i32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]),
            vy: i32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]),
            vz: i32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]),
            r: i32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]),
            g: i32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]),
            b: i32::from_le_bytes([buf[20], buf[21], buf[22], buf[23]]),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapSetup {
    pub vpx: i32,
    pub vpy: i32,
    pub vpz: i32,
    pub vrx: i32,
    pub vry: i32,
    pub vrz: i32,
    pub lights: [MapLight; 3],
    pub ambient_r: i32,
    pub ambient_g: i32,
    pub ambient_b: i32,
    pub projection: i32,
    pub region_a: [i32; 4],
    pub region_b: [i32; 4],
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<i32>,
}

impl MapSetup {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let vpx = read_i32(reader)?;
        let vpy = read_i32(reader)?;
        let vpz = read_i32(reader)?;
        let vrx = read_i32(reader)?;
        let vry = read_i32(reader)?;
        let vrz = read_i32(reader)?;
        
        let mut lights = [MapLight::default(); 3];
        for light in &mut lights {
            *light = MapLight::read(reader)?;
        }
        
        let ambient_r = read_i32(reader)?;
        let ambient_g = read_i32(reader)?;
        let ambient_b = read_i32(reader)?;
        let projection = read_i32(reader)?;
        
        let mut region_a = [0i32; 4];
        let mut region_b = [0i32; 4];
        for val in &mut region_a {
            *val = read_i32(reader)?;
        }
        for val in &mut region_b {
            *val = read_i32(reader)?;
        }
        
        let width = read_i32(reader)?;
        let height = read_i32(reader)?;
        
        let tile_count = (width * height) as usize;
        let mut tiles = Vec::with_capacity(tile_count);
        for _ in 0..tile_count {
            tiles.push(read_i32(reader)?);
        }
        
        Ok(Self {
            vpx, vpy, vpz, vrx, vry, vrz,
            lights,
            ambient_r, ambient_g, ambient_b,
            projection,
            region_a, region_b,
            width, height,
            tiles,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapImageSection {
    pub bits_per_pixel: u8,
    pub offset: usize,
    pub size: usize,
}

impl MapImageSection {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let bits_per_pixel = {
            let mut b = [0u8; 1];
            reader.read_exact(&mut b)?;
            b[0]
        };
        // padding
        let mut padding = [0u8; 3];
        reader.read_exact(&mut padding)?;
        
        let offset = read_usize(reader)?;
        let size = read_usize(reader)?;
        
        Ok(Self { bits_per_pixel, offset, size })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapSprite {
    pub tex_x: i16,
    pub tex_y: i16,
    pub width: i16,
    pub height: i16,
    pub some_x: i16,
    pub some_y: i16,
    pub some_z: i16,
    pub clut: i16,
    pub transparency: i16,
}

impl MapSprite {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let tex_x = read_i16(reader)?;
        let tex_y = read_i16(reader)?;
        let width = read_i16(reader)?;
        let height = read_i16(reader)?;
        let some_x = read_i16(reader)?;
        let some_y = read_i16(reader)?;
        let some_z = read_i16(reader)?;
        let clut = read_i16(reader)?;
        let transparency = read_i16(reader)?;
        
        Ok(Self {
            tex_x, tex_y, width, height,
            some_x, some_y, some_z,
            clut, transparency,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapObjectInstance {
    pub anim_sprites: [i16; 8],
    pub anim_times: [i16; 8],
    pub x: i16,
    pub y: i16,
    pub flag: i16,
}

impl MapObjectInstance {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut anim_sprites = [0i16; 8];
        let mut anim_times = [0i16; 8];
        
        for v in &mut anim_sprites {
            *v = read_i16(reader)?;
        }
        for v in &mut anim_times {
            *v = read_i16(reader)?;
        }
        
        let x = read_i16(reader)?;
        let y = read_i16(reader)?;
        let flag = read_i16(reader)?;
        
        Ok(Self { anim_sprites, anim_times, x, y, flag })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapObjects {
    pub sprites: Vec<MapSprite>,
    pub instances: Vec<MapObjectInstance>,
}

impl MapObjects {
    pub fn read<R: Read + Seek>(reader: &mut R, count: usize) -> Result<Self> {
        let mut sprites = Vec::with_capacity(count);
        for _ in 0..count {
            sprites.push(MapSprite::read(reader)?);
        }
        
        let instance_count = {
            let mut buf = [0u8; 2];
            reader.read_exact(&mut buf)?;
            u16::from_le_bytes(buf) as usize
        };
        
        let mut instances = Vec::with_capacity(instance_count);
        for _ in 0..instance_count {
            instances.push(MapObjectInstance::read(reader)?);
        }
        
        Ok(Self { sprites, instances })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapWarps {
    pub spawn_x: [i16; 10],
    pub spawn_y: [i16; 10],
    pub spawn_z: [i16; 10],
    pub rotation: [i16; 10],
    pub target_map: [i16; 10],
    pub target_exit: [i16; 10],
}

impl MapWarps {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut spawn_x = [0i16; 10];
        let mut spawn_y = [0i16; 10];
        let mut spawn_z = [0i16; 10];
        let mut rotation = [0i16; 10];
        let mut target_map = [0i16; 10];
        let mut target_exit = [0i16; 10];
        
        for v in &mut spawn_x { *v = read_i16(reader)?; }
        for v in &mut spawn_y { *v = read_i16(reader)?; }
        for v in &mut spawn_z { *v = read_i16(reader)?; }
        for v in &mut rotation { *v = read_i16(reader)?; }
        for v in &mut target_map { *v = read_i16(reader)?; }
        for v in &mut target_exit { *v = read_i16(reader)?; }
        
        Ok(Self { spawn_x, spawn_y, spawn_z, rotation, target_map, target_exit })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapWaypoint {
    pub x: i16,
    pub y: i16,
    pub z: i16,
}

impl MapWaypoint {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let x = read_i16(reader)?;
        let y = read_i16(reader)?;
        let z = read_i16(reader)?;
        Ok(Self { x, y, z })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapNpc {
    pub type_id: i16,
    pub follow_mode: i16,
    pub pos_x: i16,
    pub pos_y: i16,
    pub pos_z: i16,
    pub rot_x: i16,
    pub rot_y: i16,
    pub rot_z: i16,
    pub tracking_range: i16,
    pub unknown12: i16,
    pub script_id: i16,
    pub hp: i16,
    pub mp: i16,
    pub current_hp: i16,
    pub current_mp: i16,
    pub off: i16,
    pub def: i16,
    pub speed: i16,
    pub brain: i16,
    pub bits: i16,
    pub charge_mode: i16,
    pub unknown2a: i16,
    pub moves: [i16; 4],
    pub moves_prio: [i16; 4],
    pub flee_x: i16,
    pub flee_y: i16,
    pub flee_z: i16,
    pub ai_sections: [i16; 8],
    pub waypoints: Vec<MapWaypoint>,
}

impl MapNpc {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut buf = [0u8; 2];
        let mut read_i16 = || -> Result<i16> {
            reader.read_exact(&mut buf)?;
            Ok(i16::from_le_bytes(buf))
        };
        
        let type_id = read_i16()?;
        let follow_mode = read_i16()?;
        let pos_x = read_i16()?;
        let pos_y = read_i16()?;
        let pos_z = read_i16()?;
        let rot_x = read_i16()?;
        let rot_y = read_i16()?;
        let rot_z = read_i16()?;
        let tracking_range = read_i16()?;
        let unknown12 = read_i16()?;
        let script_id = read_i16()?;
        let hp = read_i16()?;
        let mp = read_i16()?;
        let current_hp = read_i16()?;
        let current_mp = read_i16()?;
        let off = read_i16()?;
        let def = read_i16()?;
        let speed = read_i16()?;
        let brain = read_i16()?;
        let bits = read_i16()?;
        let charge_mode = read_i16()?;
        let unknown2a = read_i16()?;
        
        let mut moves = [0i16; 4];
        let mut moves_prio = [0i16; 4];
        for v in &mut moves { *v = read_i16()?; }
        for v in &mut moves_prio { *v = read_i16()?; }
        
        let flee_x = read_i16()?;
        let flee_y = read_i16()?;
        let flee_z = read_i16()?;
        
        let mut ai_sections = [0i16; 8];
        for v in &mut ai_sections { *v = read_i16()?; }
        
        let waypoint_count = {
            let mut b = [0u8; 2];
            reader.read_exact(&mut b)?;
            u16::from_le_bytes(b) as usize
        };
        
        let mut waypoints = Vec::with_capacity(waypoint_count);
        for _ in 0..waypoint_count {
            waypoints.push(MapWaypoint::read(reader)?);
        }
        
        Ok(Self {
            type_id, follow_mode, pos_x, pos_y, pos_z,
            rot_x, rot_y, rot_z, tracking_range, unknown12,
            script_id, hp, mp, current_hp, current_mp,
            off, def, speed, brain, bits, charge_mode, unknown2a,
            moves, moves_prio, flee_x, flee_y, flee_z,
            ai_sections, waypoints,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapEntities {
    pub warps: MapWarps,
    pub digimon_count: i16,
    pub npcs: Vec<MapNpc>,
}

impl MapEntities {
    pub fn read<R: Read>(reader: &mut R, has_npcs: bool) -> Result<Self> {
        let warps = MapWarps::read(reader)?;
        
        let digimon_count = {
            let mut buf = [0u8; 2];
            reader.read_exact(&mut buf)?;
            i16::from_le_bytes(buf)
        };
        
        let mut npcs = Vec::new();
        if has_npcs {
            for _ in 0..MAX_NPCS {
                npcs.push(MapNpc::read(reader)?);
            }
        }
        
        Ok(Self { warps, digimon_count, npcs })
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapCollision {
    pub data: Vec<u8>,
}

impl MapCollision {
    pub fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let mut data = vec![0u8; COLLISION_SIZE * COLLISION_SIZE];
        reader.read_exact(&mut data)?;
        Ok(Self { data })
    }
    
    pub fn is_blocked(&self, x: usize, y: usize) -> bool {
        if x >= COLLISION_SIZE || y >= COLLISION_SIZE {
            return true;
        }
        let idx = y * COLLISION_SIZE + x;
        self.data[idx] & 0x80 != 0
    }
    
    pub fn trigger_id(&self, x: usize, y: usize) -> u8 {
        if x >= COLLISION_SIZE || y >= COLLISION_SIZE {
            return 0;
        }
        let idx = y * COLLISION_SIZE + x;
        self.data[idx] & 0x7F
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapFile {
    pub setup: MapSetup,
    pub images: Vec<MapImageSection>,
    pub objects: Option<MapObjects>,
    pub entities: MapEntities,
    pub collision: MapCollision,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapReadOptions {
    pub image_count_8: i32,
    pub image_count_4: i32,
    pub has_npcs: bool,
}

impl MapReadOptions {
    pub fn new(image_count_8: i32, image_count_4: i32, has_npcs: bool) -> Self {
        Self { image_count_8, image_count_4, has_npcs }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MapFileShape {
    pub image_count_8: i32,
    pub image_count_4: i32,
    pub entities_size: usize,
}

pub fn read_map_file<R: Read + Seek>(reader: &mut R, options: &MapReadOptions) -> Result<MapFile> {
    // Read offset table (6 offsets: setup, 8bpp images, 4bpp images, objects, entities, collision)
    let mut offsets = [0u32; 6];
    let mut buf = [0u8; 4];
    for i in 0..6 {
        reader.read_exact(&mut buf)?;
        offsets[i] = u32::from_le_bytes(buf);
    }
    
    // Read setup section
    reader.seek(SeekFrom::Start(offsets[0] as u64))?;
    let setup = MapSetup::read(reader)?;
    
    // Read image sections
    let mut images = Vec::new();
    
    // 8-bit images
    for _ in 0..options.image_count_8 {
        reader.seek(SeekFrom::Start(offsets[1] as u64))?;
        images.push(MapImageSection::read(reader)?);
    }
    
    // 4-bit images
    for _ in 0..options.image_count_4 {
        reader.seek(SeekFrom::Start(offsets[2] as u64))?;
        images.push(MapImageSection::read(reader)?);
    }
    
    // Read objects (only if map has images)
    let objects = if options.image_count_8 + options.image_count_4 > 0 {
        reader.seek(SeekFrom::Start(offsets[3] as u64))?;
        let sprite_count = {
            let mut b = [0u8; 2];
            reader.read_exact(&mut b)?;
            u16::from_le_bytes(b) as usize
        };
        Some(MapObjects::read(reader, sprite_count)?)
    } else {
        None
    };
    
    // Read entities
    reader.seek(SeekFrom::Start(offsets[4] as u64))?;
    let entities = MapEntities::read(reader, options.has_npcs)?;
    
    // Read collision
    reader.seek(SeekFrom::Start(offsets[5] as u64))?;
    let collision = MapCollision::read(reader)?;
    
    Ok(MapFile {
        setup,
        images,
        objects,
        entities,
        collision,
    })
}

pub fn read_map_file_shape<R: Read + Seek>(reader: &mut R) -> Result<MapFileShape> {
    let mut offsets = [0u32; 6];
    let mut buf = [0u8; 4];
    for i in 0..6 {
        reader.read_exact(&mut buf)?;
        offsets[i] = u32::from_le_bytes(buf);
    }
    
    reader.seek(SeekFrom::Start(offsets[1] as u64))?;
    
    let mut image_count_8 = 0;
    let mut image_count_4 = 0;
    
    // Read first image to determine mode
    let first_section = MapImageSection::read(reader)?;
    if first_section.bits_per_pixel == 8 {
        image_count_8 = 1;
        // Count remaining 8-bit images (they come first)
        let mut pos = reader.stream_position()?;
        while pos < offsets[2] as u64 {
            let section = MapImageSection::read(reader)?;
            if section.bits_per_pixel == 8 {
                image_count_8 += 1;
            } else {
                break;
            }
            pos = reader.stream_position()?;
        }
        // Remaining are 4-bit
        while pos < offsets[2] as u64 {
            let section = MapImageSection::read(reader)?;
            if section.bits_per_pixel == 4 {
                image_count_4 += 1;
            }
            pos = reader.stream_position()?;
        }
    } else if first_section.bits_per_pixel == 4 {
        image_count_4 = 1;
        let mut pos = reader.stream_position()?;
        while pos < offsets[2] as u64 {
            let section = MapImageSection::read(reader)?;
            if section.bits_per_pixel == 4 {
                image_count_4 += 1;
            }
            pos = reader.stream_position()?;
        }
    }
    
    // Entities section size
    let entities_size = (offsets[5] - offsets[4]) as usize;
    
    Ok(MapFileShape {
        image_count_8,
        image_count_4,
        entities_size,
    })
}

pub fn infer_map_read_options<R: Read + Seek>(reader: &mut R) -> Result<MapReadOptions> {
    let shape = read_map_file_shape(reader)?;
    
    let has_npcs = shape.entities_size > ((6 * 10 + 1) * 2);
    
    Ok(MapReadOptions::new(shape.image_count_8, shape.image_count_4, has_npcs))
}

pub fn parse_map_file(data: &[u8], options: &MapReadOptions) -> Result<MapFile> {
    let mut cursor = std::io::Cursor::new(data);
    read_map_file(&mut cursor, options)
}

pub fn map_file_path(map_id: i32, filename: &str, extension: &str) -> String {
    let dir = map_id / 15 + 1;
    format!("\\MAP\\MAP{}\\{}{}", dir, filename, extension)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_constants() {
        assert_eq!(COLLISION_SIZE, 100);
        assert_eq!(MAX_TILES, 35);
        assert_eq!(MAX_INSTANCES, 188);
        assert_eq!(MAX_NPCS, 8);
    }

    #[test]
    fn test_map_read_options() {
        let opts = MapReadOptions::new(2, 1, true);
        assert_eq!(opts.image_count_8, 2);
        assert_eq!(opts.image_count_4, 1);
        assert_eq!(opts.has_npcs, true);
    }
}
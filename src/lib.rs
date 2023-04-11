use std::f32::consts::TAU;

use cimvr_common::glam::swizzles::*;
use cimvr_common::{
    glam::{Vec2, Vec3},
    render::{Mesh, MeshHandle, Render, UploadMesh, Vertex},
    Transform,
};
use cimvr_engine_interface::{dbg, make_app_state, pkg_namespace, prelude::*, println, FrameTime};
use serde::{Deserialize, Serialize};

mod query_accel;
mod sim;

const BALL_RADIUS: f32 = 0.1;
const DT: f32 = 0.1;
const N_BALLS: usize = 10;

// All state associated with client-side behaviour
struct ClientState;

pub const CIRCLE_RDR: MeshHandle = MeshHandle::new(pkg_namespace!("Circle"));

#[derive(Component, Serialize, Deserialize, Copy, Clone, Default)]
struct LastTransform(Transform);

#[derive(Component, Serialize, Deserialize, Copy, Clone, Default)]
struct Ball {
    accel: Vec2,
}

impl UserState for ClientState {
    // Implement a constructor
    fn new(io: &mut EngineIo, _sched: &mut EngineSchedule<Self>) -> Self {
        io.send(&UploadMesh {
            mesh: circle_mesh(64, BALL_RADIUS),
            id: CIRCLE_RDR,
        });

        println!("Hello, client!");

        // NOTE: We are using the println defined by cimvr_engine_interface here, NOT the standard library!
        cimvr_engine_interface::println!("This prints");
        std::println!("But this doesn't");

        Self
    }
}

// All state associated with server-side behaviour
struct ServerState;

impl UserState for ServerState {
    // Implement a constructor
    fn new(io: &mut EngineIo, sched: &mut EngineSchedule<Self>) -> Self {
        for _ in 0..N_BALLS {
            let k = 100000;
            let mut rand = || (io.random() % k) as f32 / k as f32;
            let pos = Vec3::new(rand(), 0., rand()) * 2. - Vec3::new(1., 0., 1.);
            dbg!(pos);

            let tf = Transform::new().with_position(pos);

            io.create_entity()
                .add_component(tf)
                .add_component(LastTransform(tf))
                .add_component(Render::new(CIRCLE_RDR))
                .add_component(Synchronized)
                .build();
        }

        sched
            .add_system(Self::sim_step)
            .stage(Stage::PostUpdate)
            .query::<Transform>(Access::Read)
            .query::<LastTransform>(Access::Write)
            .subscribe::<FrameTime>()
            .build();
        Self
    }
}

impl ServerState {
    fn sim_step(&mut self, io: &mut EngineIo, query: &mut QueryResult) {
        let FrameTime { delta: dt, .. } = io.inbox_first().unwrap();

        let entities: Vec<EntityId> = query.iter().collect();
        let mut positions: Vec<Vec2> = entities
            .iter()
            .map(|&entity| query.read::<Transform>(entity).pos.xz())
            .collect();
        let last_positions: Vec<Vec2> = entities
            .iter()
            .map(|&entity| query.read::<LastTransform>(entity).0.pos.xz())
            .collect();

        sim(&mut positions, &last_positions, dt);

        // Write positions back
        for (&entity, position) in entities.iter().zip(&positions) {
            let mut tf: Transform = query.read(entity);
            query.write(entity, &LastTransform(tf));

            tf.pos.x = position.x;
            tf.pos.z = position.y;
            query.write(entity, &tf);
        }
    }
}

make_app_state!(ClientState, ServerState);

fn circle_mesh(n: usize, scale: f32) -> Mesh {
    let vertices = (0..n)
        .map(|i| TAU * i as f32 / n as f32)
        .map(|t| [t.cos() * scale, 0., t.sin() * scale])
        .map(|pos| Vertex { pos, uvw: [1.; 3] })
        .collect();

    let indices = (1..n as u32 - 1).map(|i| [i, 0, i + 1]).flatten().collect();

    Mesh { vertices, indices }
}

fn sim(positions: &mut [Vec2], last_positions: &[Vec2], dt: f32) {
    for pos in positions {
        pos.y -= dt;
    }
}

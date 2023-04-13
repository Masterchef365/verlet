use std::f32::consts::TAU;

use cimvr_common::glam::swizzles::*;
use cimvr_common::render::{RenderExtra, Primitive};
use cimvr_common::{
    glam::{Vec2, Vec3},
    render::{Mesh, MeshHandle, Render, UploadMesh, Vertex},
    Transform,
};
use cimvr_engine_interface::{dbg, make_app_state, pkg_namespace, prelude::*, println, FrameTime};
use serde::{Deserialize, Serialize};

mod query_accel;
mod sim;

const BALL_RADIUS: f32 = 0.3;
//const N_BALLS: usize = 10;
const GRAVITY: Vec2 = Vec2::new(0., -0.1);
const SUBSTEPS: usize = 1;
const CONTAINER_RADIUS: f32 = 3.;

// All state associated with client-side behaviour
struct ClientState;

pub const CIRCLE_RDR: MeshHandle = MeshHandle::new(pkg_namespace!("Circle"));
pub const BACKGROUND_CIRCLE_RDR: MeshHandle = MeshHandle::new(pkg_namespace!("Background circle"));

#[derive(Component, Serialize, Deserialize, Copy, Clone, Default)]
struct LastTransform(Transform);

#[derive(Component, Serialize, Deserialize, Copy, Clone, Default)]
struct Ball {
    accel: Vec2,
}

impl UserState for ClientState {
    fn new(io: &mut EngineIo, _sched: &mut EngineSchedule<Self>) -> Self {
        // TODO: Put this pattern (and related) into some sort of manager...
        // Any sort of higher level interface smh my head
        io.send(&UploadMesh {
            //mesh: filled_circle_mesh(20, BALL_RADIUS),
            mesh: filled_circle_mesh(120, BALL_RADIUS),
            id: CIRCLE_RDR,
        });

        io.send(&UploadMesh {
            //mesh: (20, BALL_RADIUS),
            mesh: line_circle_mesh(200, CONTAINER_RADIUS),
            id: BACKGROUND_CIRCLE_RDR,
        });

        io.create_entity()
            .add_component(Transform::new())
            .add_component(Render::new(BACKGROUND_CIRCLE_RDR).primitive(Primitive::Lines))
            .build();

        Self
    }
}

struct ServerState {
    start_time: Option<f32>,
}

impl UserState for ServerState {
    // Implement a constructor
    fn new(io: &mut EngineIo, sched: &mut EngineSchedule<Self>) -> Self {
        sched
            .add_system(Self::ball_adder)
            .stage(Stage::Update)
            .subscribe::<FrameTime>()
            .query::<Ball>(Access::Read)
            .build();

        for _ in 0..SUBSTEPS {
            sched
                .add_system(Self::gravity)
                .stage(Stage::Update)
                .query::<Ball>(Access::Write)
                .build();

            sched
                .add_system(Self::circle_constraint)
                .stage(Stage::Update)
                .query::<Ball>(Access::Read)
                .query::<Transform>(Access::Write)
                .build();

            sched
                .add_system(Self::sim_step)
                .stage(Stage::Update)
                .query::<Transform>(Access::Read)
                .query::<LastTransform>(Access::Write)
                .query::<Ball>(Access::Write)
                .subscribe::<FrameTime>()
                .build();
        }

        Self { start_time: None }
    }
}

impl ServerState {
    fn ball_adder(&mut self, io: &mut EngineIo, query: &mut QueryResult) {
        let FrameTime { delta, time } = io.inbox_first().unwrap();
        if self.start_time.is_none() {
            self.start_time = Some(time);
        }

        let time = time - self.start_time.unwrap();

        if time / 8. < query.iter().count() as f32 {
            return;
        }

        //if 

        let k = 100000;
        let mut rand = || (io.random() % k) as f32 / k as f32;
        //let pos = Vec3::new(rand(), 0., rand()) * 2. - Vec3::new(1., 0., 1.);
        let pos = Vec3::new(1., 0., -1.);

        let tf = Transform::new().with_position(pos);

        let mut extra = [0.; 4 * 4];
        for i in 0..3 {
            extra[i] = rand();
        }
        extra[3] = 1.;

        io.create_entity()
            .add_component(tf)
            .add_component(LastTransform(tf))
            .add_component(Render::new(CIRCLE_RDR).primitive(Primitive::Triangles))
            .add_component(Synchronized)
            .add_component(Ball { accel: Vec2::ZERO })
            .add_component(RenderExtra(extra))
            .build();

    }

    fn sim_step(&mut self, io: &mut EngineIo, query: &mut QueryResult) {
        let FrameTime { delta: dt, .. } = io.inbox_first().unwrap();
        let dt = dt / SUBSTEPS as f32;

        let entities: Vec<EntityId> = query.iter().collect();

        let mut positions: Vec<Vec2> = entities
            .iter()
            .map(|&entity| query.read::<Transform>(entity).pos.xz())
            .collect();

        let last_positions: Vec<Vec2> = entities
            .iter()
            .map(|&entity| query.read::<LastTransform>(entity).0.pos.xz())
            .collect();

        let accels: Vec<Vec2> = entities
            .iter()
            .map(|&entity| query.read::<Ball>(entity).accel)
            .collect();

        sim(&mut positions, &last_positions, &accels, dt);

        // Write positions back
        for (&entity, position) in entities.iter().zip(&positions) {
            let mut tf: Transform = query.read(entity);
            query.write(entity, &LastTransform(tf));

            tf.pos.x = position.x;
            tf.pos.z = position.y;
            query.write(entity, &tf);

            // Reset acceleration
            query.write(entity, &Ball { accel: Vec2::ZERO });
        }
    }

    fn gravity(&mut self, io: &mut EngineIo, query: &mut QueryResult) {
        for entity in query.iter() {
            query.modify::<Ball>(entity, |ball| ball.accel += GRAVITY);
        }
    }

    fn circle_constraint(&mut self, io: &mut EngineIo, query: &mut QueryResult) {
        for entity in query.iter() {
            query.modify::<Transform>(entity, |tf| {
                let pos = tf.pos.xz();
                let n = pos.normalize();
                let pos = n * pos.length().min(CONTAINER_RADIUS - BALL_RADIUS);

                tf.pos.x = pos.x;
                tf.pos.z = pos.y;
            });
        }
    }
}

make_app_state!(ClientState, ServerState);

fn line_circle_mesh(n: usize, scale: f32) -> Mesh {
    let vertices = (0..n)
        .map(|i| TAU * i as f32 / n as f32)
        .map(|t| [t.cos() * scale, 0., t.sin() * scale])
        .map(|pos| Vertex { pos, uvw: [1.; 3] })
        .collect();

    let indices = (0..n as u32).map(|i| [i, (i+1) % n as u32]).flatten().collect();

    Mesh { vertices, indices }
}

fn filled_circle_mesh(n: usize, scale: f32) -> Mesh {
    let vertices = (0..n)
        .map(|i| TAU * i as f32 / n as f32)
        .map(|t| [t.cos() * scale, 0., t.sin() * scale])
        .map(|pos| Vertex { pos, uvw: [1.; 3] })
        .collect();

    let indices = (1..n as u32 - 1).map(|i| [i, 0, i + 1]).flatten().collect();

    Mesh { vertices, indices }
}

fn sim(positions: &mut [Vec2], last_positions: &[Vec2], accels: &[Vec2], dt: f32) {
    // Collisions
    for i in 0..positions.len() {
        for j in (i + 1)..positions.len() {
            let diff = positions[i] - positions[j];
            let n = diff.normalize();
            let len = diff.length();
            let new_len = len.max(BALL_RADIUS * 2.);
            let displacement = (new_len - len) / 2.;
            positions[i] += displacement * n;
            positions[j] -= displacement * n;
        }
    }

    // Integrate
    for ((pos, last), accel) in positions.iter_mut().zip(last_positions).zip(accels) {
        let vel = *pos - *last;
        *pos += vel + *accel * dt.powi(2);
    }
}


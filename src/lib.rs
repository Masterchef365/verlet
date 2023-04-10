use std::f32::consts::TAU;

use cimvr_common::{render::{Mesh, Vertex, UploadMesh, MeshHandle, Render}, Transform};
use cimvr_engine_interface::{make_app_state, prelude::*, println, dbg, pkg_namespace};

// All state associated with client-side behaviour
struct ClientState;

pub const CIRCLE_RDR: MeshHandle = MeshHandle::new(pkg_namespace!("Circle"));

impl UserState for ClientState {
    // Implement a constructor
    fn new(io: &mut EngineIo, _sched: &mut EngineSchedule<Self>) -> Self {
        io.send(&UploadMesh {
            mesh: circle_mesh(64),
            id: CIRCLE_RDR,
        });

        println!("Hello, client!");

        // NOTE: We are using the println defined by cimvr_engine_interface here, NOT the standard library!
        cimvr_engine_interface::println!("This prints");
        std::println!("But this doesn't");

        io.create_entity()
            .add_component(Transform::default())
            .add_component(Render::new(CIRCLE_RDR))
            .build();

        Self
    }
}

// All state associated with server-side behaviour
struct ServerState;

impl UserState for ServerState {
    // Implement a constructor
    fn new(_io: &mut EngineIo, _sched: &mut EngineSchedule<Self>) -> Self {
        println!("Hello, server!");
        Self
    }
}

// Defines entry points for the engine to hook into.
// Calls new() for the appropriate state.
make_app_state!(ClientState, ServerState);

#[cfg(test)]
mod tests {
    #[test]
    fn im_a_test() {}
}

fn circle_mesh(n: usize) -> Mesh {
    let vertices = (0..n)
        .map(|i| TAU * i as f32 / n as f32)
        .map(|t| [t.cos(), 0., t.sin()])
        .map(|pos| Vertex { pos, uvw: [1.; 3] })
        .collect();

    let indices = (1..n as u32 - 1).map(|i| [0, i, i + 1]).flatten().collect();

    Mesh { vertices, indices }
}

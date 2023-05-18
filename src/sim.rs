use cimvr_common::glam::swizzles::*;
use cimvr_common::{
    glam::{Vec2},
};

use crate::BALL_RADIUS;
pub fn sim(positions: &mut [Vec2], last_positions: &mut [Vec2], accels: &[Vec2], dt: f32) {
    // Collisions
    for i in 0..positions.len() {
        for j in (i + 1)..positions.len() {
            let diff = positions[i] - positions[j];
            let n = diff.normalize();
            let dist = diff.length();

            //if len < BALL_RADIUS * 2. { dbg!(len); }

            let thresh = BALL_RADIUS * 2.;
            if dist < thresh {
                let displacement = (thresh - dist) / 2.;
                //displacement *= 0.95;

                positions[i] += displacement * n;
                positions[j] -= displacement * n;
                //last_positions[i] = positions[i];
                //last_positions[j] = positions[j];
                //special[i] = true;
                //special[j] = true;
            }
        }
    }

    // Integrate
    for ((pos, last), accel) in positions
        .iter_mut()
        .zip(last_positions)
        .zip(accels)
    {
        let vel = *pos - *last;
        *last = *pos;
        *pos += vel + *accel * dt.powi(2);
    }
}


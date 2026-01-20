pub struct Camera {
    pos: glam::Vec3,
    rot: glam::Quat,

    _screen: glam::Vec2,
    _fov: f32,

    proj: glam::Mat4,
    view: glam::Mat4,
}

// Computes the projection matrix: right handed screen space to
// OpenGL style NDC. We invert the viewport over in Vulkan setup
// so that the NDC works correctly.
// +X up, +Y right, +Z out of the screen.
fn compute_proj(screen: glam::Vec2, fov: f32) -> glam::Mat4 {
    let aspect = screen.x / screen.y;
    glam::Mat4::perspective_rh(fov, aspect, 0.001f32, 1000f32)
}

impl Camera {
    pub fn new(screen: glam::Vec2, fov: f32) -> Self {
        Self {
            pos: glam::Vec3::default(),
            rot: glam::Quat::default(),
            _screen: screen,
            _fov: fov,
            proj: compute_proj(screen, fov),
            view: glam::Mat4::default(),
        }
    }

    pub fn view(&self) -> glam::Mat4 {
        self.view
    }

    pub fn proj(&self) -> glam::Mat4 {
        self.proj
    }

    pub fn vp(&self) -> glam::Mat4 {
        self.proj * self.view
    }

    pub fn _transform(&self) -> (glam::Vec3, glam::Vec3) {
        (self.pos, self.rot.to_euler(glam::EulerRot::XYZ).into())
    }

    pub fn set_arcball(&mut self, target: glam::Vec3, angles: glam::Vec2, dist: f32) {
        // Calculate a transform based on being `dist` units away from `target`, rotated by angles.

        // Build our initial offset.
        let offset = glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, dist));

        // First apply our rotation. Never rotate on Z since that would look weird.
        let rotation = glam::Mat4::from_euler(glam::EulerRot::YXZ, angles.y, angles.x, 0.0);

        // And then finally move the whole setup over to our target.
        let translation = glam::Mat4::from_translation(target);

        let transform = translation * rotation * offset;
        // Extract out our position and rotation to save in the struct.
        let (_, rot, pos) = transform.to_scale_rotation_translation();

        self.pos = pos;
        self.rot = rot;
        self.view = transform.inverse();
    }
}

use cgmath::{Matrix4, Point3, Vector3, Vector4};

const OPENGL_TO_WEBGPU_MATRIX: Matrix4<f32> = Matrix4::from_cols(
    Vector4::new(1.0, 0.0, 0.0, 0.0),
    Vector4::new(0.0, 1.0, 0.0, 0.0),
    Vector4::new(0.0, 0.0, 0.5, 0.0),
    Vector4::new(0.0, 0.0, 0.5, 1.0),
);

pub struct Camera {
    pub eye: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,

    pub right: f32,
    pub top: f32,

    pub near: f32,
    pub far: f32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniform {
    view_proj: [[f32; 4]; 4],
}

impl Camera {
    pub fn build_view_proj_matrix(&self) -> Matrix4<f32> {
        let view = Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::ortho(
            -self.right,
            self.right,
            -self.top,
            self.top,
            self.near,
            self.far,
        );

        OPENGL_TO_WEBGPU_MATRIX * proj * view
    }
}

impl Uniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_proj_matrix().into();
    }
}

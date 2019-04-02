use pbs_gl as gl;
use gl::types::*;
use std::ffi::CString;
use std::ptr;

use super::shader::Shader;
use crate::core::rendering::shader::ShaderType;
use crate::core::math::matrix::Mat4;
use crate::core::math::utilities;
use crate::core::rendering::texture::Texture;
use crate::core::rendering::sampler::Sampler;

pub struct ProgramPipeline<'a> {
    id: GLuint,
    shaders: [Option<&'a Shader>; 6],
    shader_programs: [Option<GLuint>; 6]
}

impl<'a> ProgramPipeline<'a> {

    pub fn new() -> ProgramPipeline<'a> {
        let mut id: GLuint = 0;

        unsafe {
            gl::CreateProgramPipelines(1, &mut id);
        }

        ProgramPipeline {
            id,
            shaders: [None; 6],
            shader_programs: [None; 6]
        }
    }

    pub fn add_shader(&mut self, shader: &'a Shader) -> &mut Self {
        let idx = Self::shader_type_to_array_index(shader.get_type());

        if let Some(ref sdr) = self.shaders[idx] {
            eprintln!("Shader of type {:?} already exists in the program pipeline... Replacing...", shader.get_type())
        }

        self.shaders[idx] = Some(shader);

        self
    }

    pub fn build(&mut self) -> Result<&mut Self, String> {

        unsafe {
            gl::BindProgramPipeline(self.id);

            for option in self.shaders.iter() {
                match option {
                    Some(shader) => {
                        let program_id = gl::CreateProgram();

                        //must be called before linking
                        gl::ProgramParameteri(program_id, gl::PROGRAM_SEPARABLE, gl::TRUE as i32);

                        gl::AttachShader(program_id, shader.get_id());

                        gl::LinkProgram(program_id);

                        let mut link_status: GLint = 0;
                        gl::GetProgramiv(program_id, gl::LINK_STATUS, &mut link_status);

                        if link_status != gl::TRUE as i32 {
                            let mut message_size = 0;

                            gl::GetProgramiv(program_id,
                                             gl::INFO_LOG_LENGTH,
                                             &mut message_size);

                            //+1 for nul termination
                            let mut buffer =
                                Vec::with_capacity(message_size as usize + 1);

                            buffer.extend([b' ']
                                .iter()
                                .cycle()
                                .take(message_size as usize));

                            let message = CString::from_vec_unchecked(buffer);

                            gl::GetProgramInfoLog(program_id,
                                                  message_size as i32,
                                                  ptr::null_mut(),
                                                  message.as_ptr() as *mut GLchar);

                            return Err(message.to_string_lossy().into_owned());
                        }

                        let idx = Self::shader_type_to_array_index(shader.get_type());
                        self.shader_programs[idx] = Some(program_id)
                    },
                    _ => {}
                }
            }
        }

        Ok(self)
    }

    pub fn set_matrix4f(&self, name: &str, value: &Mat4, stage: ShaderType) -> &Self {

        let (program_id, location) = self.get_shader_stage_id_and_resource_location(stage,
                                                                                    gl::UNIFORM,
                                                                                    name)
            .expect("Failed to get program id or uniform location");

        unsafe {
            gl::ProgramUniformMatrix4fv(program_id,
                                        location,
                                        1,
                                        gl::FALSE,
                                        utilities::value_ptr(value))
        }

        self
    }

    pub fn set_texture(&self,
                       name: &str,
                       texture: Texture,
                       sampler: Sampler,
                       stage: ShaderType) -> &Self {
        let (_, location) = self.get_shader_stage_id_and_resource_location(stage,
                                                                                    gl::UNIFORM,
                                                                                    name)
            .expect("Failed to get program id or uniform location");

        unsafe {
            gl::BindTextureUnit(location as GLuint, texture.get_id());
            gl::BindSampler(location as GLuint, sampler.id)
        }

        self
    }

    pub fn set_integer(&self, name: &str, value: i32, stage: ShaderType) {
        let (program_id, location) = self.get_shader_stage_id_and_resource_location(stage,
                                                                                    gl::UNIFORM,
                                                                                    name)
            .expect("Failed to get program id or uniform location");

        unsafe {
            gl::ProgramUniform1i(program_id, location, value)
        }
    }

    fn shader_type_to_array_index(shader_type: ShaderType) -> usize {
        match shader_type {
            ShaderType::Vertex => 0,
            ShaderType::TesselationControl => 1,
            ShaderType::TesselationEvaluation => 2,
            ShaderType::Geometry => 3,
            ShaderType::Fragment => 4,
            ShaderType::Compute => 5,
        }
    }

    fn get_shader_stage_id_and_resource_location(&self,
                                                 stage: ShaderType,
                                                 resource_type: GLenum,
                                                 name: &str) -> Result<(GLuint, GLint), String> {
        let program_index = Self::shader_type_to_array_index(stage);

        let program_id = match self.shader_programs[program_index] {
            Some(id) => id,
            _ => {
                return Err(format!("Shader of type {:?} is not present in the program pipeline", stage));
            }
        };

        let c_str = CString::new(name).unwrap();
        let location = unsafe { gl::GetProgramResourceLocation(program_id,
                                                               resource_type,
                                                               c_str.as_ptr() as *const GLchar) };

        if location < 0 {
            return Err(format!("Uniform: {} is not active or does not exist in shader stage {:?} with ID {}", name, stage, program_id))
        }

        Ok((program_id, location))
    }
}

impl<'a> Drop for ProgramPipeline<'a> {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgramPipelines(1, &mut self.id)
        }
    }
}

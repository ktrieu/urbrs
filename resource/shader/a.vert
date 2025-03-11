#version 450

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 color;

layout( push_constant ) uniform constants
{
	mat4 mvp;
} push_constants;

layout (location = 0) out vec3 outColor;
layout (location = 1) out vec3 outNorm;

void main() 
{
	outNorm = normalize(position);
	// output the position of each vertex
	gl_Position = push_constants.mvp * vec4(position, 1.0f);
	outColor = color;
}
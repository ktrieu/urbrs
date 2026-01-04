#version 450

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;

layout( push_constant ) uniform constants
{
	mat4 mvp;
} push_constants;

layout (location = 0) out vec3 outPosition;
layout (location = 1) out vec3 outNormal;

void main() 
{
	outNormal = normal;
	vec4 projectedPosition = push_constants.mvp * vec4(position, 1.0);
	
	outPosition = projectedPosition.xyz;
	gl_Position = projectedPosition;
}
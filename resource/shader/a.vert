#version 450

layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;

layout(set = 0, binding = 0) uniform GlobalSceneData {
	mat4 view;
	mat4 proj;
	mat4 vp;
} globalSceneData;

layout (location = 0) out vec3 ssPosition;
layout (location = 1) out vec3 ssNormal;

void main() 
{
	ssNormal = (vec4(normal, 0.0) * globalSceneData.view).xyz;
	ssPosition = (vec4(position, 1.0) * globalSceneData.view).xyz;
	
	vec4 projectedPosition = globalSceneData.vp * vec4(position, 1.0);
	gl_Position = projectedPosition;
}
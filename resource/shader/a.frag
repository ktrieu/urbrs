#version 450

layout (location = 0) in vec3 ssPosition;
layout (location = 1) in vec3 ssNormal;

layout(set = 0, binding = 0) uniform GlobalSceneData {
	mat4 view;
	mat4 proj;
	mat4 vp;
} globalSceneData;

layout (location = 0) out vec4 outFragColor;

const vec3 LIGHT_DIR = normalize(vec3(-1, -1, -1));

const vec3 ALBEDO = vec3(1.0);
const float AMBIENT = 0.01;

void main() 
{
	vec3 ssLightDir = normalize((vec4(LIGHT_DIR, 0.0) * globalSceneData.view).xyz);

	vec3 normal = normalize(ssNormal);
	float diffuseFac = clamp(dot(normal, ssLightDir), 0.0, 1.0);
	vec3 diffuse = ALBEDO * diffuseFac;

	vec3 ambient = AMBIENT * ALBEDO;

	outFragColor = vec4(diffuse + ambient, 1.0);
}
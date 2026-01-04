#version 450

// shader input
layout (location = 0) in vec3 inPosition;
layout (location = 1) in vec3 inNormal;

// output write
layout (location = 0) out vec4 outFragColor;

const vec3 LIGHT_DIR = normalize(vec3(-1, -1, -1));

const vec3 ALBEDO = vec3(1.0);
const float AMBIENT = 0.01;

void main() 
{
	vec3 normal = normalize(inNormal);
	float diffuseFac = clamp(dot(normal, LIGHT_DIR), 0.0, 1.0);
	vec3 diffuse = ALBEDO * diffuseFac;

	vec3 ambient = AMBIENT * ALBEDO;

	outFragColor = vec4(diffuse + ambient, 1.0);
}
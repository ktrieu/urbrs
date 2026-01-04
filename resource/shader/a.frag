#version 450

// shader input
layout (location = 0) in vec3 inPosition;
layout (location = 1) in vec3 inNormal;

// output write
layout (location = 0) out vec4 outFragColor;

const vec3 LIGHT_DIR = normalize(vec3(-1, -1, -1));

void main() 
{
	outFragColor = abs(vec4(inNormal, 1.0f));
}
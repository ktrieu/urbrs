#version 450

// shader input
layout (location = 0) in vec3 inColor;
layout (location = 1) in vec3 inNorm;

// output write
layout (location = 0) out vec4 outFragColor;

const vec3 LIGHT_DIR = normalize(vec3(-1, -1, -1));

void main() 
{
	outFragColor = vec4(inNorm, 1.0f);
}
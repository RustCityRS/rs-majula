uniform float time;
uniform float scale;

varying vec3 wvVertex;
varying float waterDepth;

void main() {
	waterDepth = gl_MultiTexCoord1.x;
	//calculate the vertex's position in eye space
	vec4 ecVertex = gl_ModelViewMatrix*gl_Vertex;
	//calculate the world space vertex vector
	wvVertex.x =  dot(gl_ModelViewMatrix[0].xyz, ecVertex.xyz);
	wvVertex.y = -dot(gl_ModelViewMatrix[1].xyz, ecVertex.xyz);
	wvVertex.z = -dot(gl_ModelViewMatrix[2].xyz, ecVertex.xyz);
	//calculate the texture coordinates from the vertex's position
	gl_TexCoord[0].x = dot(gl_TextureMatrix[0][0], gl_MultiTexCoord0)*scale;
	gl_TexCoord[0].y = dot(gl_TextureMatrix[0][1], gl_MultiTexCoord0)*scale;
	gl_TexCoord[0].z = time;
	gl_TexCoord[0].w = 1.0;
	//assign the fog coordinate
	gl_FogFragCoord = 1.0-clamp((gl_Fog.end+ecVertex.z)*gl_Fog.scale, 0.0, 1.0);
	//calculate the final clip space position
	gl_Position = ftransform();
}
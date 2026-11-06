!!ARBvp1.0
ATTRIB  iPos         = vertex.position;
ATTRIB  iNormal      = vertex.normal;
ATTRIB  iColour      = vertex.color;
ATTRIB  iTexCoord    = vertex.texcoord[0];
ATTRIB  iWaterDepth  = vertex.texcoord[1];
OUTPUT  oPos         = result.position;
OUTPUT  oColour      = result.color;
OUTPUT  oTexCoord0   = result.texcoord[0];
OUTPUT  oTexCoord1   = result.texcoord[1];
OUTPUT  oFogCoord    = result.fogcoord;
PARAM   fogParams    = program.local[0];
PARAM   waterPlane   = program.local[1];
PARAM   tMatrix[4]   = { state.matrix.texture[0] };
PARAM   pMatrix[4]   = { state.matrix.projection };
PARAM   mvMatrix[4]  = { state.matrix.modelview };
TEMP    viewPos, viewNormal, fogFactor, colour, ndotl;
# Calculate the vertex's eye coordinates
DP4   viewPos.x, mvMatrix[0], iPos;
DP4   viewPos.y, mvMatrix[1], iPos;
DP4   viewPos.z, mvMatrix[2], iPos;
DP4   viewPos.w, mvMatrix[3], iPos;
# Calculate a fog factor based on the vertex's depth into the screen
SUB   fogFactor.x, -viewPos.z, fogParams.x;
MUL   fogFactor.x, fogFactor.x, 0.001953125;
# Calculate a fog factor based on the vertex's depth below the water
MAD   fogFactor.y, iWaterDepth.x, fogParams.z, fogParams.w;
# We're going to shift the vertex towards the water plane where the distance fog will cover up what we're doing.
SUB   fogFactor.z, -viewPos.z, fogParams.y;
MUL   fogFactor.z, fogFactor.z, 0.00390625;
# try and avoid distance fogging in the shallows
MUL   fogFactor.x, fogFactor.x, fogFactor.y;
# Clamp the fog factors
MIN   fogFactor, fogFactor, 1;
MAX   fogFactor, fogFactor, 0;
# Shift the vertex towards the water plane dependent on it's depth into the screen
MUL   fogFactor.z, fogFactor.z, iWaterDepth.x;
MAD   viewPos.xyz, waterPlane.xyzw, fogFactor.zzzz, viewPos.xyzw;
# Use the foggiest of the two fog factors
MAX   oTexCoord1.xyz, fogFactor.xxxx, fogFactor.yyyy;
MOV   oTexCoord1.w, 1;
# Calculate the view space normal
DP3   viewNormal.x, mvMatrix[0], iNormal;
DP3   viewNormal.y, mvMatrix[1], iNormal;
DP3   viewNormal.z, mvMatrix[2], iNormal;
# Calculate the diffuse terms for the light
DP3   ndotl.x, viewNormal, state.light[0].position;
DP3   ndotl.y, viewNormal, state.light[1].position;
# Clamp the lighting values
MAX   ndotl, ndotl, 0;
# Calculate the lighting for the vertex, start with the ambient
MOV   colour, state.lightmodel.ambient;
# Add the two lights contributions
MAD   colour, state.light[0].diffuse, ndotl.xxxx, colour;
MAD   colour, state.light[1].diffuse, ndotl.yyyy, colour;
# Perform the final diffuse modulation
MUL   oColour, iColour, colour;	
# Calculate the vertex's clip coordinates
DP4   oPos.x, pMatrix[0], viewPos;
DP4   oPos.y, pMatrix[1], viewPos;
DP4   oPos.z, pMatrix[2], viewPos;
DP4   oPos.w, pMatrix[3], viewPos;
# The fog coord is simply the vertex's camera space depth
MOV   oFogCoord.x, viewPos.z;
# Transfer the base texture coordinate
DP3   oTexCoord0.x, tMatrix[0], iTexCoord;
DP3   oTexCoord0.y, tMatrix[1], iTexCoord;
MOV   oTexCoord0.zw, iTexCoord;
END!!ARBvp1.0
ATTRIB  iPos              = vertex.position;
ATTRIB  iTexCoord         = vertex.texcoord[0];
OUTPUT  oPos              = result.position;
OUTPUT  oColour           = result.color;
OUTPUT  oTexCoord0        = result.texcoord[0];
PARAM   time              = program.local[0];
PARAM   fogPlanes         = program.local[1];
PARAM   fogColor          = state.fog.color;
PARAM   fogParams         = state.fog.params;
PARAM   mvMatrix[4]       = { state.matrix.modelview };
PARAM   pMatrix[4]        = { state.matrix.projection };
PARAM   textureMatrix[4]  = { state.matrix.texture[0] };
TEMP viewPos, texCoord, fogFactor;
#Calculate the vertex's clip coordinates
DP4   viewPos.x, mvMatrix[0], iPos;
DP4   viewPos.y, mvMatrix[1], iPos;
DP4   viewPos.z, mvMatrix[2], iPos;
DP4   viewPos.w, mvMatrix[3], iPos;
#Transform the base texture coordinate
DP3   texCoord.x, textureMatrix[0], iTexCoord;
DP3   texCoord.y, textureMatrix[1], iTexCoord;
MUL   oTexCoord0.xy, texCoord, 0.25;
MOV   oTexCoord0.zw, time.xxxw;
#Transform the vertex position to clip space
DP4   oPos.x, pMatrix[0], viewPos;
DP4   oPos.y, pMatrix[1], viewPos;
DP4   oPos.z, pMatrix[2], viewPos;
DP4   oPos.w, pMatrix[3], viewPos;
#Use the fog colour
MOV   oColour.xyz, fogColor;
#Alpha the surface to give the impression of fog
ADD   fogFactor, viewPos.zzzz, fogParams.zzzz;
SUB   fogFactor, 1, fogFactor;
MUL   oColour.w, fogFactor.xxxx, fogParams.wwww;
END
!!ARBvp1.0
ATTRIB  iPos         = vertex.position;
ATTRIB  iNormal      = vertex.normal;
ATTRIB  iColour      = vertex.color;
ATTRIB  iTexCoord    = vertex.texcoord[0];
OUTPUT  oPos         = result.position;
OUTPUT  oColour      = result.color;
OUTPUT  oTexCoord0   = result.texcoord[0];
OUTPUT  oTexCoord1   = result.texcoord[1];
OUTPUT  oFogCoord    = result.fogcoord;
PARAM   fogParams    = program.local[0];
PARAM   waterPlane   = program.local[1];
PARAM   pMatrix[4]   = { state.matrix.projection };
PARAM   mvMatrix[4]  = { state.matrix.modelview };
PARAM   texMatrix[4] = { state.matrix.texture[0] };
TEMP    viewPos, viewNormal, fogFactor, depth, colour, ndotl;
# Calculate the vertex's eye coordinates
DP4   viewPos.x, mvMatrix[0], iPos;
DP4   viewPos.y, mvMatrix[1], iPos;
DP4   viewPos.z, mvMatrix[2], iPos;
DP4   viewPos.w, mvMatrix[3], iPos;
# Calculate a fog factor based on the vertex's depth into the screen
SUB   fogFactor.x, -viewPos.z, fogParams.x;
MUL   fogFactor.x, fogFactor.x, 0.001953125;
# Calculate a fog factor based on the vertex's depth below the water
DP4   depth, waterPlane, viewPos;
MAD   fogFactor.y, -depth, fogParams.z, fogParams.w;
# We're going to shift the vertex towards the water plane where the distance fog will cover up what we're doing.
SUB   fogFactor.z, -viewPos.z, fogParams.y;
MUL   fogFactor.z, fogFactor.z, 0.00390625;
# Clamp the fog factors
MIN   fogFactor, fogFactor, 1;
MAX   fogFactor, fogFactor, 0;
# Shift the vertex towards the water plane dependant on it's depth into the screen
MUL   fogFactor.z, fogFactor.z, -depth;
MAD   viewPos.xyz, waterPlane.xyzw, fogFactor.zzzz, viewPos.xyzw;
# Use the foggiest of the two fog factors
MAX   oTexCoord1.xyz, fogFactor.xxxx, fogFactor.yyyy;
MOV   oTexCoord1.w, 1;
# Calculate the view space normal
DP3   viewNormal.x, mvMatrix[0], iNormal;
DP3   viewNormal.y, mvMatrix[1], iNormal;
DP3   viewNormal.z, mvMatrix[2], iNormal;
# Calculate the diffuse terms for the light
DP3   ndotl.x, viewNormal, state.light[0].position;
DP3   ndotl.y, viewNormal, state.light[1].position;
# Clamp the lighting values
MAX   ndotl, ndotl, 0;
# Calculate the lighting for the vertex, start with the ambient
MOV   colour, state.lightmodel.ambient;
# Add the two lights contributions
MAD   colour, state.light[0].diffuse, ndotl.xxxx, colour;
MAD   colour, state.light[1].diffuse, ndotl.yyyy, colour;
# Perform the final diffuse modulation
MUL   oColour, iColour, colour;
# Calculate the vertex's clip coordinates
DP4   oPos.x, pMatrix[0], viewPos;
DP4   oPos.y, pMatrix[1], viewPos;
DP4   oPos.z, pMatrix[2], viewPos;
DP4   oPos.w, pMatrix[3], viewPos;
# The fog coord is simply the vertex's camera space depth
MOV   oFogCoord.x, viewPos.z;
# Transfer the base texture coordinate
DP4   oTexCoord0.x, texMatrix[0], iTexCoord;
DP4   oTexCoord0.y, texMatrix[1], iTexCoord;
DP4   oTexCoord0.z, texMatrix[2], iTexCoord;
MOV   oTexCoord0.w, 1;
END!!ARBvp1.0
ATTRIB  iPos         = vertex.position;
ATTRIB  iColour      = vertex.color;
ATTRIB  iTexCoord    = vertex.texcoord[0];
OUTPUT  oPos         = result.position;
OUTPUT  oColour      = result.color;
OUTPUT  oTexCoord0   = result.texcoord[0];
OUTPUT  oTexCoord1   = result.texcoord[1];
OUTPUT  oFogCoord    = result.fogcoord;
PARAM   fogParams    = program.local[0];
PARAM   waterPlane   = program.local[1];
PARAM   pMatrix[4]   = { state.matrix.projection };
PARAM   mvMatrix[4]  = { state.matrix.modelview };
PARAM   texMatrix[4] = { state.matrix.texture[0] };
TEMP    viewPos, fogFactor, depth;
# Calculate the vertex's eye coordinates
DP4   viewPos.x, mvMatrix[0], iPos;
DP4   viewPos.y, mvMatrix[1], iPos;
DP4   viewPos.z, mvMatrix[2], iPos;
DP4   viewPos.w, mvMatrix[3], iPos;
# Calculate a fog factor based on the vertex's depth into the screen
SUB   fogFactor.x, -viewPos.z, fogParams.x;
MUL   fogFactor.x, fogFactor.x, 0.001953125;
# Calculate a fog factor based on the vertex's depth below the water
DP4   depth, waterPlane, viewPos;
MAD   fogFactor.y, -depth, fogParams.z, fogParams.w;
# We're going to shift the vertex towards the water plane where the distance fog will cover up what we're doing.
SUB   fogFactor.z, -viewPos.z, fogParams.y;
MUL   fogFactor.z, fogFactor.z, 0.00390625;
# Clamp the fog factors
MIN   fogFactor, fogFactor, 1;
MAX   fogFactor, fogFactor, 0;
# Shift the vertex towards the water plane dependant on it's depth into the screen
MUL   fogFactor.z, fogFactor.z, -depth;
MAD   viewPos.xyz, waterPlane.xyzw, fogFactor.zzzz, viewPos.xyzw;
# Use the foggiest of the two fog factors
MAX   oTexCoord1.xyz, fogFactor.xxxx, fogFactor.yyyy;
MOV   oTexCoord1.w, 1;
# Transfer the base colour
MOV   oColour, iColour;
# Calculate the vertex's clip coordinates
DP4   oPos.x, pMatrix[0], viewPos;
DP4   oPos.y, pMatrix[1], viewPos;
DP4   oPos.z, pMatrix[2], viewPos;
DP4   oPos.w, pMatrix[3], viewPos;
# The fog coord is simply the vertex's camera space depth
MOV   oFogCoord.x, viewPos.z;
# Transform the base texture coordinates
DP4   oTexCoord0.x, texMatrix[0], iTexCoord;
DP4   oTexCoord0.y, texMatrix[1], iTexCoord;
DP4   oTexCoord0.z, texMatrix[2], iTexCoord;
MOV   oTexCoord0.w, 1;
END!!ARBvp1.0
ATTRIB  iPos         = vertex.position;
ATTRIB  iColour      = vertex.color;
ATTRIB  iTexCoord    = vertex.texcoord[0];
ATTRIB  iWaterDepth  = vertex.texcoord[1];
OUTPUT  oPos         = result.position;
OUTPUT  oColour      = result.color;
OUTPUT  oTexCoord0   = result.texcoord[0];
OUTPUT  oTexCoord1   = result.texcoord[1];
OUTPUT  oFogCoord    = result.fogcoord;
PARAM   fogParams    = program.local[0];
PARAM   waterPlane   = program.local[1];
PARAM   tMatrix[4]   = { state.matrix.texture[0] };
PARAM   pMatrix[4]   = { state.matrix.projection };
PARAM   mvMatrix[4]  = { state.matrix.modelview };
TEMP    viewPos, fogFactor;
# Calculate the vertex's eye coordinates
DP4   viewPos.x, mvMatrix[0], iPos;
DP4   viewPos.y, mvMatrix[1], iPos;
DP4   viewPos.z, mvMatrix[2], iPos;
DP4   viewPos.w, mvMatrix[3], iPos;
# Calculate a fog factor based on the vertex's depth into the screen
SUB   fogFactor.x, -viewPos.z, fogParams.x;
MUL   fogFactor.x, fogFactor.x, 0.001953125;
# Calculate a fog factor based on the vertex's depth below the water
MAD   fogFactor.y, iWaterDepth.x, fogParams.z, fogParams.w;
# We're going to shift the vertex towards the water plane where the distance fog will cover up what we're doing.
SUB   fogFactor.z, -viewPos.z, fogParams.y;
MUL   fogFactor.z, fogFactor.z, 0.00390625;
# try and avoid distance fogging in the shallows
MUL   fogFactor.x, fogFactor.x, fogFactor.y;
# Clamp the fog factors
MIN   fogFactor, fogFactor, 1;
MAX   fogFactor, fogFactor, 0;
# Shift the vertex towards the water plane dependent on it's depth into the screen
MUL   fogFactor.z, fogFactor.z, iWaterDepth.x;
MAD   viewPos.xyz, waterPlane.xyzw, fogFactor.zzzz, viewPos.xyzw;
# Use the foggiest of the two fog factors
MAX   oTexCoord1.xyz, fogFactor.xxxx, fogFactor.yyyy;
MOV   oTexCoord1.w, 1;
# Transfer the base colour
MOV   oColour, iColour;
# Calculate the vertex's clip coordinates
DP4   oPos.x, pMatrix[0], viewPos;
DP4   oPos.y, pMatrix[1], viewPos;
DP4   oPos.z, pMatrix[2], viewPos;
DP4   oPos.w, pMatrix[3], viewPos;
# The fog coord is simply the vertex's camera space depth
MOV   oFogCoord.x, viewPos.z;
# Transfer the base texture coordinate
DP3   oTexCoord0.x, tMatrix[0], iTexCoord;
DP3   oTexCoord0.y, tMatrix[1], iTexCoord;
MOV   oTexCoord0.zw, iTexCoord;
ENDvarying vec3 wvVertex;	
varying float waterDepth;	

uniform vec3 sunDir;
uniform vec4 sunColour;
uniform float sunExponent;

uniform float breakWaterDepth;
uniform float breakWaterOffset;

uniform sampler3D normalSampler;
uniform samplerCube envMapSampler;

void main() {
	//retrieve the normal and the noise
	vec4 wnNormal = texture3D(normalSampler, gl_TexCoord[0].xyz).rgba;
	//shift the normal into the range [-1,1]
	wnNormal.xyz = 2.0*wnNormal.xyz-1.0;
	
	//Environment map lookup
 	//----------------------
	//normalise the world space vertex vector
	vec3 wnVector = normalize(wvVertex);
	//calculate the world space reflection vector
	vec3 wnReflection = reflect(wnVector, wnNormal.xyz);
	//use the reflection vector to retrieve a colour from the environment map
	vec3 envColour = textureCube(envMapSampler, wnReflection).rgb;
	
	//Specular calculation
	//--------------------
	vec4 specularColour = sunColour*pow(clamp(dot(sunDir, wnReflection), 0.0, 1.0), sunExponent);
	
	//Break water calculation
	//-----------------------						
	float shoreFactor = clamp(waterDepth/breakWaterDepth-breakWaterOffset*wnNormal.w, 0.0, 1.0);

	//Combining
	//---------
	//calculate a fresnel(ish) term
	float ndote = dot(wnVector, wnNormal.xyz);
	float fresnel = pow(1.0-abs(ndote), 2.0);
	//use the fresnel and distance to shore to determine the opacity
	vec4 surfaceColour = vec4(envColour, fresnel*shoreFactor)+specularColour*shoreFactor;
	
	//finally apply fogging
	gl_FragColor = vec4(mix(surfaceColour.rgb, gl_Fog.color.rgb, gl_FogFragCoord), surfaceColour.a);
}uniform float time;
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
}  ¯ÿÿú	  °ÿÿýG   DÿÿýSÿÿýZ
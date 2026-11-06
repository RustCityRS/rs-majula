varying vec3 wvVertex;	
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
}



struct VS_OUT {
    vec4 position;
    vec3 texCoord;
    vec4 colour;
};



VS_OUT ret_0;
vec4 r0015;
float x0017;
float TMP18;
float b0023;
vec2 r0025;
vec4 v0025;
uniform vec4 modelViewProjectionMatrix[4];
uniform vec2 textureMatrix[4];
uniform vec4 fogPlanes;

 void main()
{

    VS_OUT output;

    r0015 = gl_Vertex.x*modelViewProjectionMatrix[0];
    r0015 = r0015 + gl_Vertex.y*modelViewProjectionMatrix[1];
    r0015 = r0015 + gl_Vertex.z*modelViewProjectionMatrix[2];
    r0015 = r0015 + gl_Vertex.w*modelViewProjectionMatrix[3];
    x0017 = fogPlanes.x + (r0015.z + 1.00000000E+00)*fogPlanes.y;
    b0023 = min(1.00000000E+00, x0017);
    TMP18 = max(0.00000000E+00, b0023);
    output.texCoord.z = 1.00000000E+00 - TMP18;
    v0025 = vec4(gl_MultiTexCoord0.x, gl_MultiTexCoord0.y, 0.00000000E+00, 1.00000000E+00);
    r0025 = v0025.x*textureMatrix[0];
    r0025 = r0025 + v0025.y*textureMatrix[1];
    r0025 = r0025 + v0025.z*textureMatrix[2];
    r0025 = r0025 + v0025.w*textureMatrix[3];
    output.texCoord.xy = r0025;
    ret_0.position = r0015;
    ret_0.texCoord = output.texCoord;
    ret_0.colour = gl_Color;
    gl_FrontColor = gl_Color;
    gl_Position = r0015;
    gl_TexCoord[0].xyz = output.texCoord;
    return;
} 
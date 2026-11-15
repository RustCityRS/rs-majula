


struct VS_OUT {
    vec4 position;
    vec3 texCoord;
    float fog;
};



VS_OUT ret_0;
vec4 r0011;
vec2 r0013;
vec4 v0013;
float x0015;
float TMP16;
float b0021;
uniform vec4 modelViewProjectionMatrix[4];
uniform vec2 textureMatrix[4];
uniform vec4 fogPlanes;
uniform float time;

 void main()
{

    VS_OUT output;

    r0011 = gl_Vertex.x*modelViewProjectionMatrix[0];
    r0011 = r0011 + gl_Vertex.y*modelViewProjectionMatrix[1];
    r0011 = r0011 + gl_Vertex.z*modelViewProjectionMatrix[2];
    r0011 = r0011 + gl_Vertex.w*modelViewProjectionMatrix[3];
    v0013 = vec4(gl_MultiTexCoord0.x, gl_MultiTexCoord0.y, 0.00000000E+00, 1.00000000E+00);
    r0013 = v0013.x*textureMatrix[0];
    r0013 = r0013 + v0013.y*textureMatrix[1];
    r0013 = r0013 + v0013.z*textureMatrix[2];
    r0013 = r0013 + v0013.w*textureMatrix[3];
    output.texCoord.xy = r0013;
    output.texCoord.z = time;
    x0015 = fogPlanes.x + (r0011.z + 1.00000000E+00)*fogPlanes.y;
    b0021 = min(1.00000000E+00, x0015);
    TMP16 = max(0.00000000E+00, b0021);
    output.fog = 1.00000000E+00 - TMP16;
    ret_0.position = r0011;
    ret_0.texCoord = output.texCoord;
    ret_0.fog = output.fog;
    gl_FrontColor.x = output.fog;
    gl_Position = r0011;
    gl_TexCoord[0].xyz = output.texCoord;
    return;
} 
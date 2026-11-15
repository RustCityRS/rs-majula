


struct VS_OUT {
    vec4 position2;
    vec4 colour;
    vec2 texCoord;
    float fog;
    vec4 texCoord2;
    vec3 objectPos1;
};





VS_OUT ret_0;
vec4 r0030;
vec4 r0032;
vec3 v0038;
float x0040;
float TMP43;
float a0044;
float TMP47;
float b0052;
vec3 TMP55;
vec3 b0060;
vec3 r0064;
vec3 lighting0066;
float x0070;
float TMP71;
float b0076;
vec4 r0078;
vec2 r0080;
vec4 v0080;
vec3 TMP85;
vec3 TMP86;
vec3 TMP87;
uniform vec3 sunColour;
uniform vec3 ambientColour;
uniform vec3 sunDirection;
uniform vec2 textureMatrix[4];
uniform vec4 modelMatrix[4];
uniform vec4 modelViewMatrix[4];
uniform vec4 projectionMatrix[4];
uniform vec4 fogPlanes;
uniform vec4 fogParams;
uniform vec4 waterPlane;
uniform float time;
uniform vec2 waterParams;
uniform vec3 eyePosition;

 void main()
{

    VS_OUT output;
    vec3 fogFactor;
    vec4 viewPos;
    float t;
    vec3 hitPos;

    fogFactor = vec3( 0.00000000E+00, 0.00000000E+00, 0.00000000E+00);
    r0030 = gl_Vertex.x*modelMatrix[0];
    r0030 = r0030 + gl_Vertex.y*modelMatrix[1];
    r0030 = r0030 + gl_Vertex.z*modelMatrix[2];
    r0030 = r0030 + gl_Vertex.w*modelMatrix[3];
    r0032 = gl_Vertex.x*modelViewMatrix[0];
    r0032 = r0032 + gl_Vertex.y*modelViewMatrix[1];
    r0032 = r0032 + gl_Vertex.z*modelViewMatrix[2];
    r0032 = r0032 + gl_Vertex.w*modelViewMatrix[3];
    fogFactor.xz = (-fogPlanes.xz - r0032.zz)*fogPlanes.yw;
    t = (r0030.y - eyePosition.y)/(0.00000000E+00 - eyePosition.y);
    hitPos = eyePosition + t*(r0030.xyz - eyePosition);
    v0038 = r0030.xyz - hitPos;
    a0044 = dot(v0038, v0038);
    TMP43 = 1.00000000E+00/inversesqrt(a0044);
    x0040 = TMP43*fogParams.x;
    b0052 = min(1.00000000E+00, x0040);
    TMP47 = max(0.00000000E+00, b0052);
    output.fog = TMP47*fogParams.y;
    fogFactor.z = 1.00000000E+00 - fogFactor.z;
    if (lit) {         fogFactor.x = 0.00000000E+00;
    }     b0060 = min(vec3( 1.00000000E+00, 1.00000000E+00, 1.00000000E+00), fogFactor);
    TMP55 = max(vec3( 0.00000000E+00, 0.00000000E+00, 0.00000000E+00), b0060);
    viewPos.xyz = r0032.xyz + waterPlane.xyz*TMP55.z*gl_MultiTexCoord1.x;
    if (lit) {         TMP85.x = modelMatrix[0].x;
        TMP85.y = modelMatrix[0].y;
        TMP85.z = modelMatrix[0].z;
        r0064 = gl_Normal.x*TMP85;
        TMP86.x = modelMatrix[1].x;
        TMP86.y = modelMatrix[1].y;
        TMP86.z = modelMatrix[1].z;
        r0064 = r0064 + gl_Normal.y*TMP86;
        TMP87.x = modelMatrix[2].x;
        TMP87.y = modelMatrix[2].y;
        TMP87.z = modelMatrix[2].z;
        r0064 = r0064 + gl_Normal.z*TMP87;
        x0070 = dot(r0064, sunDirection)*5.00000000E-01 + 5.00000000E-01;
        b0076 = min(1.00000000E+00, x0070);
        TMP71 = max(0.00000000E+00, b0076);
        lighting0066 = TMP71*(sunColour + ambientColour*5.00000000E-01) + ambientColour*5.00000000E-01;
        output.colour.xyz = gl_Color.xyz*lighting0066;
        output.colour.w = gl_Color.w;
    } else {
        output.colour = gl_Color;
    }     r0078 = viewPos.x*projectionMatrix[0];
    r0078 = r0078 + viewPos.y*projectionMatrix[1];
    r0078 = r0078 + viewPos.z*projectionMatrix[2];
    r0078 = r0078 + r0032.w*projectionMatrix[3];
    v0080 = vec4(gl_MultiTexCoord0.x, gl_MultiTexCoord0.y, 0.00000000E+00, 1.00000000E+00);
    r0080 = v0080.x*textureMatrix[0];
    r0080 = r0080 + v0080.y*textureMatrix[1];
    r0080 = r0080 + v0080.z*textureMatrix[2];
    r0080 = r0080 + v0080.w*textureMatrix[3];
    r0030.xyz;
    output.texCoord2.xy = (r0030.xz/1.02400000E+03)*waterParams.y;
    output.texCoord2.z = time;
    output.texCoord2.w = gl_MultiTexCoord1.x;
    ret_0.position2 = r0078;
    ret_0.colour = output.colour;
    ret_0.texCoord = r0080;
    ret_0.fog = output.fog;
    ret_0.texCoord2 = output.texCoord2;
    ret_0.objectPos1 = r0030.xyz;
    gl_FrontColor = output.colour;
    gl_TexCoord[1].x = output.fog;
    gl_TexCoord[3].xyz = r0030.xyz;
    gl_Position = r0078;
    gl_TexCoord[2] = output.texCoord2;
    gl_TexCoord[0].xy = r0080;
    return;
} 
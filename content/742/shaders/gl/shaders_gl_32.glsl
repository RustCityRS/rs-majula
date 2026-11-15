


struct VS_OUT {
    vec4 Position;
    vec4 Colour;
    vec2 TexCoord;
};

VS_OUT ret_0;
vec4 r0006;
vec2 r0008;
uniform vec4 WVPMatrix[4];
uniform vec2 TexCoordMatrix[4];

 void main()
{


    r0006 = gl_Vertex.x*WVPMatrix[0];
    r0006 = r0006 + gl_Vertex.y*WVPMatrix[1];
    r0006 = r0006 + gl_Vertex.z*WVPMatrix[2];
    r0006 = r0006 + gl_Vertex.w*WVPMatrix[3];
    r0008 = gl_MultiTexCoord0.x*TexCoordMatrix[0];
    r0008 = r0008 + gl_MultiTexCoord0.y*TexCoordMatrix[1];
    r0008 = r0008 + gl_MultiTexCoord0.z*TexCoordMatrix[2];
    r0008 = r0008 + gl_MultiTexCoord0.w*TexCoordMatrix[3];
    ret_0.Position = r0006;
    ret_0.Colour = gl_Color;
    ret_0.TexCoord = r0008;
    gl_FrontColor = gl_Color;
    gl_Position = r0006;
    gl_TexCoord[0].xy = r0008;
    return;
} 
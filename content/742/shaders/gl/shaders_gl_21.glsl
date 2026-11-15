


struct VS_OUT {
    vec4 Position;
    vec4 TexCoord;
};



VS_OUT ret_0;
vec4 r0010;
vec2 r0012;
vec2 r0014;
uniform vec4 WVPMatrix[4];
uniform vec2 SpriteTexCoordMatrix[4];
uniform vec2 MaskTexCoordMatrix[4];

 void main()
{

    VS_OUT Out;

    r0010 = gl_Vertex.x*WVPMatrix[0];
    r0010 = r0010 + gl_Vertex.y*WVPMatrix[1];
    r0010 = r0010 + gl_Vertex.z*WVPMatrix[2];
    r0010 = r0010 + gl_Vertex.w*WVPMatrix[3];
    r0012 = gl_MultiTexCoord0.x*SpriteTexCoordMatrix[0];
    r0012 = r0012 + gl_MultiTexCoord0.y*SpriteTexCoordMatrix[1];
    r0012 = r0012 + gl_MultiTexCoord0.z*SpriteTexCoordMatrix[2];
    r0012 = r0012 + gl_MultiTexCoord0.w*SpriteTexCoordMatrix[3];
    Out.TexCoord.xy = r0012;
    if (Masked) {         r0014 = gl_MultiTexCoord0.x*MaskTexCoordMatrix[0];
        r0014 = r0014 + gl_MultiTexCoord0.y*MaskTexCoordMatrix[1];
        r0014 = r0014 + gl_MultiTexCoord0.z*MaskTexCoordMatrix[2];
        r0014 = r0014 + gl_MultiTexCoord0.w*MaskTexCoordMatrix[3];
        Out.TexCoord.zw = r0014;
    } else {
        Out.TexCoord.zw = vec2( 0.00000000E+00, 0.00000000E+00);
    }     ret_0.Position = r0010;
    ret_0.TexCoord = Out.TexCoord;
    gl_Position = r0010;
    gl_TexCoord[0] = Out.TexCoord;
    return;
} 
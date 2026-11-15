


struct VS_OUT {
    vec4 position;
    vec4 colour;
    vec3 texCoord;
};

VS_OUT ret_0;
vec4 r0011;
vec4 r0013;
uniform vec4 WorldMatrix[4];
uniform vec4 WVPMatrix[4];
uniform vec4 UGenerationPlane;
uniform vec4 VGenerationPlane;
uniform vec4 Time;

 void main()
{

    VS_OUT output;

    r0011 = gl_Vertex.x*WVPMatrix[0];
    r0011 = r0011 + gl_Vertex.y*WVPMatrix[1];
    r0011 = r0011 + gl_Vertex.z*WVPMatrix[2];
    r0011 = r0011 + gl_Vertex.w*WVPMatrix[3];
    r0013 = gl_Vertex.x*WorldMatrix[0];
    r0013 = r0013 + gl_Vertex.y*WorldMatrix[1];
    r0013 = r0013 + gl_Vertex.z*WorldMatrix[2];
    r0013 = r0013 + gl_Vertex.w*WorldMatrix[3];
    output.texCoord.x = dot(r0013, UGenerationPlane);
    output.texCoord.y = dot(r0013, VGenerationPlane);
    output.texCoord.z = Time.x;
    ret_0.position = r0011;
    ret_0.colour = gl_Color;
    ret_0.texCoord = output.texCoord;
    gl_FrontColor = gl_Color;
    gl_Position = r0011;
    gl_TexCoord[0].xyz = output.texCoord;
    return;
} 
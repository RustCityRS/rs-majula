


struct VS_OUT {
    vec4 position;
    vec4 colour;
};



VS_OUT ret_0;
vec4 r0006;
vec4 r0008;
uniform vec4 modelMatrix[4];
uniform vec4 projectionMatrix[4];

 void main()
{


    r0006 = gl_Vertex.x*modelMatrix[0];
    r0006 = r0006 + gl_Vertex.y*modelMatrix[1];
    r0006 = r0006 + gl_Vertex.z*modelMatrix[2];
    r0006 = r0006 + gl_Vertex.w*modelMatrix[3];
    r0008 = r0006.x*projectionMatrix[0];
    r0008 = r0008 + r0006.y*projectionMatrix[1];
    r0008 = r0008 + r0006.z*projectionMatrix[2];
    r0008 = r0008 + r0006.w*projectionMatrix[3];
    ret_0.position = r0008;
    ret_0.colour = gl_Color;
    gl_FrontColor = gl_Color;
    gl_Position = r0008;
    return;
} 
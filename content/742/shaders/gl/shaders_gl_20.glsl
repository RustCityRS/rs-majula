
vec4 ret_0;
vec4 r0003;
uniform vec4 projectionMatrix[4];

 void main()
{


    r0003 = gl_Vertex.x*projectionMatrix[0];
    r0003 = r0003 + gl_Vertex.y*projectionMatrix[1];
    r0003 = r0003 + gl_Vertex.z*projectionMatrix[2];
    r0003 = r0003 + gl_Vertex.w*projectionMatrix[3];
    ret_0 = r0003;
    gl_Position = r0003;
    return;
} 
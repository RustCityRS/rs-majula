


struct VS_OUT {
    vec4 Position;
    vec2 TexCoord;
    vec4 Colour;
};



VS_OUT ret_0;

 void main()
{


    ret_0.Position = gl_Vertex;
    ret_0.TexCoord = gl_MultiTexCoord0.xy;
    ret_0.Colour = gl_Color;
    gl_FrontColor = gl_Color;
    gl_Position = gl_Vertex;
    gl_TexCoord[0].xy = gl_MultiTexCoord0.xy;
    return;
} 
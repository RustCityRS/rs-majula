




vec4 ret_0;
uniform sampler2D DiffuseSampler;
uniform vec4 DiffuseColour;

 void main()
{


    ret_0 = texture2D(DiffuseSampler, gl_TexCoord[0].xy)*DiffuseColour*gl_Color;
    gl_FragColor = ret_0;
    return;
} 
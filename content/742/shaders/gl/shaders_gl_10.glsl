






vec4 ret_0;
uniform vec4 fogColour;
uniform sampler3D causticSampler3D;

 void main()
{

    vec4 texColour;

    texColour = texture3D(causticSampler3D, gl_TexCoord[0].xyz);
    ret_0 = texColour + gl_Color.x*(fogColour - texColour);
    gl_FragColor = ret_0;
    return;
} 
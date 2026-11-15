




vec4 ret_0;
uniform sampler3D billowSampler3D;

 void main()
{

    vec4 noise;

    noise = texture3D(billowSampler3D, gl_TexCoord[0].xyz);
    ret_0 = noise*gl_Color;
    gl_FragColor = ret_0;
    return;
} 
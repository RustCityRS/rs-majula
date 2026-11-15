




vec4 ret_0;
uniform sampler2D billowSampler2D;

 void main()
{

    vec4 noise;

    noise = texture2D(billowSampler2D, gl_TexCoord[0].xy);
    ret_0 = noise*gl_Color;
    gl_FragColor = ret_0;
    return;
} 
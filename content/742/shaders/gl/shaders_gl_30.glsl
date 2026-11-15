






vec4 ret_0;
uniform vec4 fogColour;
uniform sampler2D causticSampler2D;

 void main()
{

    vec4 texColour;

    texColour = texture2D(causticSampler2D, gl_TexCoord[0].xy);
    ret_0 = texColour + gl_Color.x*(fogColour - texColour);
    gl_FragColor = ret_0;
    return;
} 
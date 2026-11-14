




struct PS_OUT {
    vec4 Colour;
};

PS_OUT ret_0;
uniform sampler2D SpriteSampler;

 void main()
{

    PS_OUT Out;

    Out.Colour = texture2D(SpriteSampler, gl_TexCoord[0].xy)*gl_Color;
    ret_0.Colour = Out.Colour;
    gl_FragColor = Out.Colour;
    return;
} 
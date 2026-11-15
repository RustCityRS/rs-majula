




struct PS_OUT {
    vec4 Colour;
};

PS_OUT ret_0;
uniform sampler2D SpriteSampler;
uniform sampler2D MaskSampler;
uniform vec4 MulColour;
uniform vec4 AddColour;

 void main()
{

    PS_OUT Out;
    vec4 DiffuseColour;
    float Mask1;

    DiffuseColour = texture2D(SpriteSampler, gl_TexCoord[0].xy);
    if (AlphaTex) {         DiffuseColour = DiffuseColour.wwww;
    }     DiffuseColour = DiffuseColour*MulColour + AddColour;
    if (Masked) {         Mask1 = texture2D(MaskSampler, gl_TexCoord[0].zw).w;
        Out.Colour = vec4(DiffuseColour.x, DiffuseColour.y, DiffuseColour.z, DiffuseColour.w*Mask1);
    } else {
        Out.Colour = DiffuseColour;
    }     ret_0.Colour = Out.Colour;
    gl_FragColor = Out.Colour;
    return;
} 
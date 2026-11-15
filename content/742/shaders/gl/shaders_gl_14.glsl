




vec4 ret_0;
uniform sampler2D sceneTex;

 void main()
{


    ret_0 = texture2D(sceneTex, gl_TexCoord[0].xy);
    gl_FragColor = ret_0;
    return;
} 
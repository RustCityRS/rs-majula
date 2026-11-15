




vec4 ret_0;
uniform sampler2D screen;

 void main()
{

    vec3 TMP0;

    TMP0 = texture2D(screen, gl_TexCoord[0].xy).xyz;
    ret_0 = vec4(TMP0.x, TMP0.y, TMP0.z, 1.00000000E+00);
    gl_FragColor = ret_0;
    return;
} 
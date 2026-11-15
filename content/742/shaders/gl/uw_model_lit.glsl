






vec4 ret_0;
float t0012;
float b0016;
uniform vec4 fogColour;

 void main()
{

    vec4 foggedColour;

    t0012 = gl_TexCoord[0].z*fogColour.w;
    foggedColour = gl_Color + t0012*(fogColour - gl_Color);
    b0016 = max(foggedColour.y, foggedColour.z);
    foggedColour = foggedColour/vec4(max(foggedColour.x, b0016), max(foggedColour.x, b0016), max(foggedColour.x, b0016), max(foggedColour.x, b0016));
    ret_0 = foggedColour;
    gl_FragColor = foggedColour;
    return;
} 
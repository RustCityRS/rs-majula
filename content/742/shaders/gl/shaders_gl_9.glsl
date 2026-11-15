
struct VS_IN {
    int index;
};

struct VS_OUT {
    vec4 position;
    vec2 texCoord;
};

VS_OUT ret_0;
VS_IN input1;
uniform vec4 PosAndTexCoords[3];

 void main()
{

    VS_OUT output;
    vec4 coords;

    input1.index = int(gl_MultiTexCoord0.x);
    coords = PosAndTexCoords[input1.index];
    output.position = vec4(coords.x, coords.y, 0.00000000E+00, 1.00000000E+00);
    output.texCoord = vec2(coords.z, -coords.w);
    ret_0.position = output.position;
    ret_0.texCoord = output.texCoord;
    gl_Position = output.position;
    gl_TexCoord[0].xy = output.texCoord;
    return;
} 
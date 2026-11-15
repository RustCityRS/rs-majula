version=vs_2_0
ctab.flags=33024
ctab.constant=TexCoordMatrix,c4,2,18,type0
ctab.constant=WVPMatrix,c0,4,2,type1
ctab.string=TexCoordMatrix,"TexCoordMatrix"
ctab.padding=171,1
ctab.type=type0,matrix_columns,float,4,2,1
ctab.string=WVPMatrix,"WVPMatrix"
ctab.padding=171,2
ctab.type=type1,matrix_columns,float,4,4,1
ctab.string=target,"vs_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
dcl position0, v0.xyzw
dcl color0, v1.xyzw
dcl texcoord0, v2.xyzw
dp4 oPos.x, v0.xyzw, c0.xyzw
dp4 oPos.y, v0.xyzw, c1.xyzw
dp4 oPos.z, v0.xyzw, c2.xyzw
dp4 oPos.w, v0.xyzw, c3.xyzw
dp4 oT0.x, v2.xyzw, c4.xyzw
dp4 oT0.y, v2.xyzw, c5.xyzw
mov oD0.xyzw, v1.xyzw

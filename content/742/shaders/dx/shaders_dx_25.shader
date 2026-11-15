version=vs_2_0
ctab.flags=33024
ctab.constant=modelMatrix,c0,4,2,type0
ctab.constant=projectionMatrix,c4,4,18,type0
ctab.string=modelMatrix,"modelMatrix"
ctab.type=type0,matrix_columns,float,4,4,1
ctab.string=projectionMatrix,"projectionMatrix"
ctab.string=target,"vs_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
ctab.padding=171,3
dcl position0, v0.xyzw
dcl color0, v1.xyzw
dp4 r0.x, v0.xyzw, c0.xyzw
dp4 r0.y, v0.xyzw, c1.xyzw
dp4 r0.z, v0.xyzw, c2.xyzw
dp4 r0.w, v0.xyzw, c3.xyzw
dp4 oPos.x, r0.xyzw, c4.xyzw
dp4 oPos.y, r0.xyzw, c5.xyzw
dp4 oPos.z, r0.xyzw, c6.xyzw
dp4 oPos.w, r0.xyzw, c7.xyzw
mov oD0.xyzw, v1.xyzw

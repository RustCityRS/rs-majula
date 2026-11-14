version=vs_2_0
ctab.flags=33024
ctab.constant=projectionMatrix,c0,4,2,type0
ctab.string=projectionMatrix,"projectionMatrix"
ctab.padding=171,3
ctab.type=type0,matrix_columns,float,4,4,1
ctab.string=target,"vs_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
dcl position0, v0.xyzw
dp4 oPos.x, v0.xyzw, c0.xyzw
dp4 oPos.y, v0.xyzw, c1.xyzw
dp4 oPos.z, v0.xyzw, c2.xyzw
dp4 oPos.w, v0.xyzw, c3.xyzw

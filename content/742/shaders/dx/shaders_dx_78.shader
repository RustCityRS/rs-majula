version=vs_2_0
ctab.flags=33024
ctab.constant=Time,c10,1,42,type0
ctab.constant=UGenerationPlane,c8,1,34,type0
ctab.constant=VGenerationPlane,c9,1,38,type0
ctab.constant=WVPMatrix,c4,4,18,type1
ctab.constant=WorldMatrix,c0,4,2,type1
ctab.string=Time,"Time"
ctab.padding=171,3
ctab.type=type0,vector,float,1,4,1
ctab.string=UGenerationPlane,"UGenerationPlane"
ctab.string=VGenerationPlane,"VGenerationPlane"
ctab.string=WVPMatrix,"WVPMatrix"
ctab.type=type1,matrix_columns,float,4,4,1
ctab.string=WorldMatrix,"WorldMatrix"
ctab.string=target,"vs_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
dcl position0, v0.xyzw
dcl color0, v1.xyzw
dp4 oPos.x, v0.xyzw, c4.xyzw
dp4 oPos.y, v0.xyzw, c5.xyzw
dp4 oPos.z, v0.xyzw, c6.xyzw
dp4 oPos.w, v0.xyzw, c7.xyzw
dp4 r0.x, v0.xyzw, c0.xyzw
dp4 r0.y, v0.xyzw, c1.xyzw
dp4 r0.z, v0.xyzw, c2.xyzw
dp4 r0.w, v0.xyzw, c3.xyzw
dp4 oT0.x, r0.xyzw, c8.xyzw
dp4 oT0.y, r0.xyzw, c9.xyzw
mov oD0.xyzw, v1.xyzw
mov oT0.z, c10.xxxx

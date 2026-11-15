version=vs_2_0
ctab.flags=33024
ctab.constant=AmbientColour,c10,1,42,type0
ctab.constant=AntiSunColour,c9,1,38,type0
ctab.constant=DistanceFogPlane,c6,1,26,type1
ctab.constant=SunColour,c8,1,34,type0
ctab.constant=SunDir,c7,1,30,type0
ctab.constant=TexCoordMatrix,c4,2,18,type2
ctab.constant=WVPMatrix,c0,4,2,type3
ctab.string=AmbientColour,"AmbientColour"
ctab.padding=171,2
ctab.type=type0,vector,float,1,3,1
ctab.string=AntiSunColour,"AntiSunColour"
ctab.string=DistanceFogPlane,"DistanceFogPlane"
ctab.padding=171,1
ctab.type=type1,vector,float,1,4,1
ctab.string=SunColour,"SunColour"
ctab.string=SunDir,"SunDir"
ctab.string=TexCoordMatrix,"TexCoordMatrix"
ctab.type=type2,matrix_columns,float,4,2,1
ctab.string=WVPMatrix,"WVPMatrix"
ctab.padding=171,2
ctab.type=type3,matrix_columns,float,4,4,1
ctab.string=target,"vs_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c11.xyzw, 0, 1, 0, 0
dcl position0, v0.xyzw
dcl texcoord0, v1.xyzw
dcl normal0, v2.xyzw
dcl color0, v3.xyzw
dcl texcoord1, v4.xyzw
dp4 oPos.x, v0.xyzw, c0.xyzw
dp4 oPos.y, v0.xyzw, c1.xyzw
dp4 oPos.z, v0.xyzw, c2.xyzw
dp4 oPos.w, v0.xyzw, c3.xyzw
dp4 oT0.x, v1.xyzw, c4.xyzw
dp4 oT0.y, v1.xyzw, c5.xyzw
dp3 r0.x, v2.xyzw, c7.xyzw
max r0.y, r0.xxxx, c11.xxxx
max r0.x, -r0.xxxx, c11.xxxx
min r0.xy, r0.xyzw, c11.yyyy
mov r1.xyz, c8.xyzw
mad r0.yzw, r0.yyyy, r1.xxyz, c10.xxyz
mad r0.xyz, r0.xxxx, c9.xyzw, r0.yzww
mul oD0.xyz, r0.xyzw, v3.xyzw
dp4 r0.x, v0.xyzw, c6.xyzw
max r0.x, r0.xxxx, c11.xxxx
min oT0.w, r0.xxxx, c11.yyyy
mov oT0.z, v4.xxxx
mov oD0.w, v3.wwww
mov oD1.xyzw, c11.xxxy
mov oT1.xyz, c11.xxxx
mov oT2.xyzw, c11.xxxx
mov oT3.xyz, c11.xxxx

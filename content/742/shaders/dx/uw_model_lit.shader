version=vs_2_0
ctab.flags=33024
ctab.constant=AmbientColour,c11,1,46,type0
ctab.constant=AntiSunColour,c10,1,42,type0
ctab.constant=DistanceFogPlane,c7,1,30,type1
ctab.constant=EyePos,c6,1,26,type0
ctab.constant=SunColour,c9,1,38,type0
ctab.constant=SunDir,c8,1,34,type0
ctab.constant=TexCoordMatrix,c4,2,18,type2
ctab.constant=WVPMatrix,c0,4,2,type3
ctab.string=AmbientColour,"AmbientColour"
ctab.padding=171,2
ctab.type=type0,vector,float,1,3,1
ctab.string=AntiSunColour,"AntiSunColour"
ctab.string=DistanceFogPlane,"DistanceFogPlane"
ctab.padding=171,1
ctab.type=type1,vector,float,1,4,1
ctab.string=EyePos,"EyePos"
ctab.string=SunColour,"SunColour"
ctab.string=SunDir,"SunDir"
ctab.string=TexCoordMatrix,"TexCoordMatrix"
ctab.padding=171,1
ctab.type=type2,matrix_columns,float,4,2,1
ctab.string=WVPMatrix,"WVPMatrix"
ctab.padding=171,2
ctab.type=type3,matrix_columns,float,4,4,1
ctab.string=target,"vs_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c12.xyzw, 0, 1, 0, 0
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
mov oT2.xyz, -c8.xyzw
add r0.xyz, -v0.xyzw, c6.xyzw
dp3 r0.w, r0.xyzw, r0.xyzw
rsq r0.w, r0.wwww
mul oT1.xyz, r0.wwww, r0.xyzw
dp3 r0.x, v2.xyzw, v2.xyzw
rsq r0.x, r0.xxxx
mul oT3.xyz, r0.xxxx, v2.xyzw
dp3 r0.x, v2.xyzw, c8.xyzw
max r0.y, r0.xxxx, c12.xxxx
max r0.x, -r0.xxxx, c12.xxxx
min r0.xy, r0.xyzw, c12.yyyy
mov r1.xyz, c9.xyzw
mad r0.yzw, r0.yyyy, r1.xxyz, c11.xxyz
mad r0.xyz, r0.xxxx, c10.xyzw, r0.yzww
mul oD0.xyz, r0.xyzw, v3.xyzw
dp4 r0.x, v0.xyzw, c7.xyzw
max r0.x, r0.xxxx, c12.xxxx
min r0.x, r0.xxxx, c12.yyyy
add oT2.w, -r0.xxxx, c12.yyyy
mov oT0.w, r0.xxxx
mov oT0.z, v4.xxxx
mov oD0.w, v3.wwww
mov oD1.xyzw, c12.xxxy

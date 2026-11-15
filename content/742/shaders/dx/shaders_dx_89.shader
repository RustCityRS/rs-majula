version=vs_2_0
ctab.flags=33024
ctab.constant=AmbientColour,c14,1,58,type0
ctab.constant=AntiSunColour,c13,1,54,type0
ctab.constant=DistanceFogColour,c8,1,34,type0
ctab.constant=DistanceFogPlane,c7,1,30,type1
ctab.constant=EyePos,c6,1,26,type0
ctab.constant=HeightFogColour,c10,1,42,type0
ctab.constant=HeightFogPlane,c9,1,38,type1
ctab.constant=PointLightsDiffuseColour,c22,3,90,type2
ctab.constant=PointLightsPosAndRadiusSq,c18,3,74,type3
ctab.constant=SunColour,c12,1,50,type0
ctab.constant=SunDir,c11,1,46,type0
ctab.constant=TexCoordMatrix,c4,2,18,type4
ctab.constant=WVPMatrix,c0,4,2,type5
ctab.constant=WorldMatrix,c15,3,62,type6
ctab.string=AmbientColour,"AmbientColour"
ctab.padding=171,2
ctab.type=type0,vector,float,1,3,1
ctab.string=AntiSunColour,"AntiSunColour"
ctab.string=DistanceFogColour,"DistanceFogColour"
ctab.string=DistanceFogPlane,"DistanceFogPlane"
ctab.padding=171,3
ctab.type=type1,vector,float,1,4,1
ctab.string=EyePos,"EyePos"
ctab.string=HeightFogColour,"HeightFogColour"
ctab.string=HeightFogPlane,"HeightFogPlane"
ctab.string=PointLightsDiffuseColour,"PointLightsDiffuseColour"
ctab.padding=171,1
ctab.type=type2,vector,float,1,4,4
ctab.string=PointLightsPosAndRadiusSq,"PointLightsPosAndRadiusSq"
ctab.padding=171,2
ctab.type=type3,vector,float,1,4,4
ctab.string=SunColour,"SunColour"
ctab.string=SunDir,"SunDir"
ctab.string=TexCoordMatrix,"TexCoordMatrix"
ctab.type=type4,matrix_columns,float,4,2,1
ctab.string=WVPMatrix,"WVPMatrix"
ctab.padding=171,2
ctab.type=type5,matrix_columns,float,4,4,1
ctab.string=WorldMatrix,"WorldMatrix"
ctab.type=type6,matrix_columns,float,3,3,1
ctab.string=target,"vs_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c21.xyzw, 0, 1, 0, 0
dcl position0, v0.xyzw
dcl texcoord0, v1.xyzw
dcl normal0, v2.xyzw
dcl color0, v3.xyzw
dp4 oPos.x, v0.xyzw, c0.xyzw
dp4 oPos.y, v0.xyzw, c1.xyzw
dp4 oPos.z, v0.xyzw, c2.xyzw
dp4 oPos.w, v0.xyzw, c3.xyzw
dp4 oT0.x, v1.xyzw, c4.xyzw
dp4 oT0.y, v1.xyzw, c5.xyzw
add r0.xyz, -v0.xyzw, c6.xyzw
nrm r1.xyz, r0.xyzw
dp3 r0.x, r1.xyzw, v2.xyzw
add r0.x, r0.xxxx, r0.xxxx
mad r0.xyz, v2.xyzw, -r0.xxxx, r1.xyzw
dp3 oT1.x, r0.xyzw, c15.xyzw
dp3 oT1.y, r0.xyzw, c16.xyzw
dp3 oT1.z, r0.xyzw, c17.xyzw
add r0.xyz, -v0.xyzw, c18.xyzw
dp3 r0.w, r0.xyzw, r0.xyzw
rsq r1.x, r0.wwww
rcp r0.w, r0.wwww
mul r0.w, r0.wwww, c18.wwww
mul r0.xyz, r0.xyzw, r1.xxxx
dp3 r0.x, v2.xyzw, r0.xyzw
max r0.x, r0.xxxx, c21.xxxx
min r0.x, r0.xxxx, c21.yyyy
mul r0.x, r0.xxxx, r0.wwww
dp3 r0.y, v2.xyzw, c11.xyzw
max r0.z, r0.yyyy, c21.xxxx
max r0.y, -r0.yyyy, c21.xxxx
min r0.yz, r0.xyzw, c21.yyyy
mov r1.xyz, c12.xyzw
mad r1.xyz, r0.zzzz, r1.xyzw, c14.xyzw
mad r0.yzw, r0.yyyy, c13.xxyz, r1.xxyz
mad r0.xyz, c22.xyzw, r0.xxxx, r0.yzww
add r1.xyz, -v0.xyzw, c19.xyzw
dp3 r0.w, r1.xyzw, r1.xyzw
rsq r1.w, r0.wwww
rcp r0.w, r0.wwww
mul r0.w, r0.wwww, c19.wwww
mul r1.xyz, r1.wwww, r1.xyzw
dp3 r1.x, v2.xyzw, r1.xyzw
max r1.x, r1.xxxx, c21.xxxx
min r1.x, r1.xxxx, c21.yyyy
mul r0.w, r0.wwww, r1.xxxx
mad r0.xyz, c23.xyzw, r0.wwww, r0.xyzw
add r1.xyz, -v0.xyzw, c20.xyzw
dp3 r0.w, r1.xyzw, r1.xyzw
rsq r1.w, r0.wwww
rcp r0.w, r0.wwww
mul r0.w, r0.wwww, c20.wwww
mul r1.xyz, r1.wwww, r1.xyzw
dp3 r1.x, v2.xyzw, r1.xyzw
max r1.x, r1.xxxx, c21.xxxx
min r1.x, r1.xxxx, c21.yyyy
mul r0.w, r0.wwww, r1.xxxx
mad r0.xyz, c24.xyzw, r0.wwww, r0.xyzw
mul r0.xyz, r0.xyzw, v3.xyzw
dp4 r0.w, v0.xyzw, c9.xyzw
max r0.w, r0.wwww, c21.xxxx
min r0.w, r0.wwww, c21.yyyy
add r1.x, -r0.wwww, c21.yyyy
mul r1.yzw, r0.wwww, c10.xxyz
mul r0.xyz, r0.xyzw, r1.xxxx
dp4 r0.w, v0.xyzw, c7.xyzw
max r0.w, r0.wwww, c21.xxxx
min r0.w, r0.wwww, c21.yyyy
add r2.x, -r0.wwww, c21.yyyy
mul r2.yzw, r0.wwww, c8.xxyz
mad oD1.xyz, r1.yzww, r2.xxxx, r2.yzww
mul oD0.xyz, r0.xyzw, r2.xxxx
mul oD1.w, r1.xxxx, r2.xxxx
mov oT0.zw, c21.xxxx
mov oD0.w, v3.wwww
mov oT2.xyzw, c21.xxxx
mov oT3.xyz, c21.xxxx

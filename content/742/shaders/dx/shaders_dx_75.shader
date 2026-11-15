version=vs_2_0
ctab.flags=33024
ctab.constant=DistanceFogColour,c8,1,34,type0
ctab.constant=DistanceFogPlane,c7,1,30,type1
ctab.constant=EyePosAndTime,c6,1,26,type1
ctab.constant=HeightFogColour,c10,1,42,type0
ctab.constant=HeightFogPlane,c9,1,38,type1
ctab.constant=TexCoordMatrix,c4,2,18,type2
ctab.constant=WVPMatrix,c0,4,2,type3
ctab.string=DistanceFogColour,"DistanceFogColour"
ctab.padding=171,2
ctab.type=type0,vector,float,1,3,1
ctab.string=DistanceFogPlane,"DistanceFogPlane"
ctab.padding=171,3
ctab.type=type1,vector,float,1,4,1
ctab.string=EyePosAndTime,"EyePosAndTime"
ctab.string=HeightFogColour,"HeightFogColour"
ctab.string=HeightFogPlane,"HeightFogPlane"
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
dcl texcoord1, v2.xyzw
dp4 oPos.x, v0.xyzw, c0.xyzw
dp4 oPos.y, v0.xyzw, c1.xyzw
dp4 oPos.z, v0.xyzw, c2.xyzw
dp4 oPos.w, v0.xyzw, c3.xyzw
dp4 oT0.x, v1.xyzw, c4.xyzw
dp4 oT0.y, v1.xyzw, c5.xyzw
add r0.xyz, -v0.xyzw, c6.xyzw
dp3 r0.w, r0.xyzw, r0.xyzw
rsq r0.w, r0.wwww
mul oT1.xyz, r0.wwww, r0.xyzw
dp4 r0.x, v0.xyzw, c9.xyzw
max r0.x, r0.xxxx, c11.xxxx
min r0.x, r0.xxxx, c11.yyyy
add r0.y, -r0.xxxx, c11.yyyy
mul r0.xzw, r0.xxxx, c10.xyyz
dp4 r1.x, v0.xyzw, c7.xyzw
max r1.x, r1.xxxx, c11.xxxx
min r1.x, r1.xxxx, c11.yyyy
add r1.y, -r1.xxxx, c11.yyyy
mul r1.xzw, r1.xxxx, c8.xyyz
mad oD1.xyz, r0.xzww, r1.yyyy, r1.xzww
mul oD0.xyz, r0.yyyy, r1.yyyy
mov oT0.z, c6.wwww
mov oT0.w, v2.xxxx
mov oD0.w, c11.yyyy
mov oD1.w, c11.xxxx

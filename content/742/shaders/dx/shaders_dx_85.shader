version=vs_2_0
ctab.flags=33024
ctab.constant=DistanceFogColour,c7,1,30,type0
ctab.constant=DistanceFogPlane,c6,1,26,type1
ctab.constant=HeightFogColour,c9,1,38,type0
ctab.constant=HeightFogPlane,c8,1,34,type1
ctab.constant=TexCoordMatrix,c4,2,18,type2
ctab.constant=WVPMatrix,c0,4,2,type3
ctab.string=DistanceFogColour,"DistanceFogColour"
ctab.padding=171,2
ctab.type=type0,vector,float,1,3,1
ctab.string=DistanceFogPlane,"DistanceFogPlane"
ctab.padding=171,3
ctab.type=type1,vector,float,1,4,1
ctab.string=HeightFogColour,"HeightFogColour"
ctab.string=HeightFogPlane,"HeightFogPlane"
ctab.string=TexCoordMatrix,"TexCoordMatrix"
ctab.padding=171,2
ctab.type=type2,matrix_columns,float,4,2,1
ctab.string=WVPMatrix,"WVPMatrix"
ctab.padding=171,2
ctab.type=type3,matrix_columns,float,4,4,1
ctab.string=target,"vs_2_0"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
def c10.xyzw, 0, 1, 0, 0
dcl position0, v0.xyzw
dcl texcoord0, v1.xyzw
dcl color0, v2.xyzw
dp4 oPos.x, v0.xyzw, c0.xyzw
dp4 oPos.y, v0.xyzw, c1.xyzw
dp4 oPos.z, v0.xyzw, c2.xyzw
dp4 oPos.w, v0.xyzw, c3.xyzw
dp4 oT0.x, v1.xyzw, c4.xyzw
dp4 oT0.y, v1.xyzw, c5.xyzw
dp4 r0.x, v0.xyzw, c8.xyzw
max r0.x, r0.xxxx, c10.xxxx
min r0.x, r0.xxxx, c10.yyyy
add r0.y, -r0.xxxx, c10.yyyy
mul r0.xzw, r0.xxxx, c9.xyyz
mul r1.xyz, r0.yyyy, v2.xyzw
dp4 r1.w, v0.xyzw, c6.xyzw
max r1.w, r1.wwww, c10.xxxx
min r1.w, r1.wwww, c10.yyyy
add r2.x, -r1.wwww, c10.yyyy
mul r2.yzw, r1.wwww, c7.xxyz
mad oD1.xyz, r0.xzww, r2.xxxx, r2.yzww
mul oD0.xyz, r1.xyzw, r2.xxxx
mul oD1.w, r0.yyyy, r2.xxxx
mov oT0.zw, c10.xxxx
mov oD0.w, v2.wwww
mov oT1.xyz, c10.xxxx
mov oT2.xyzw, c10.xxxx
mov oT3.xyz, c10.xxxx

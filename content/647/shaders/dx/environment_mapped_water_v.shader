version=vs_1_1
ctab.flags=295168
ctab.constant=fogPlanes,c16,1,66,type0
ctab.constant=invViewMatrix,c4,3,18,type1
ctab.constant=modelViewMatrix,c0,4,2,type2
ctab.constant=projectionMatrix,c8,4,34,type2
ctab.constant=scale,c15,1,62,type3
ctab.constant=textureMatrix,c12,2,50,type4
ctab.constant=time,c14,1,58,type3
ctab.string=fogPlanes,"fogPlanes"
ctab.padding=171,2
ctab.type=type0,vector,float,1,2,1
ctab.string=invViewMatrix,"invViewMatrix"
ctab.padding=171,2
ctab.type=type1,matrix_columns,float,3,3,1
ctab.string=modelViewMatrix,"modelViewMatrix"
ctab.type=type2,matrix_columns,float,4,4,1
ctab.string=projectionMatrix,"projectionMatrix"
ctab.string=scale,"scale"
ctab.padding=171,1
ctab.type=type3,scalar,float,1,1,1
ctab.string=textureMatrix,"textureMatrix"
ctab.padding=171,2
ctab.type=type4,matrix_columns,float,4,2,1
ctab.string=time,"time"
ctab.string=target,"vs_1_1"
ctab.string=creator,"Microsoft (R) HLSL Shader Compiler 9.27.952.3022"
ctab.padding=171,3
def c7.xyzw, 1, 0, 0, 0
dcl position0, v0.xyzw
dcl texcoord0, v1.xyzw
dcl texcoord1, v2.xyzw
mov r0.x, c4.xxxx
mov r0.y, c5.xxxx
mov r0.z, c6.xxxx
dp4 r1.x, v0.xyzw, c0.xyzw
dp4 r1.y, v0.xyzw, c1.xyzw
dp4 r1.z, v0.xyzw, c2.xyzw
dp3 oT2.x, r1.xyzw, r0.xyzw
mov r0.x, c4.yyyy
mov r0.y, c5.yyyy
mov r0.z, c6.yyyy
dp3 r0.x, r1.xyzw, r0.xyzw
mov oT2.y, -r0.xxxx
mov r0.x, c4.zzzz
mov r0.y, c5.zzzz
mov r0.z, c6.zzzz
dp3 r0.x, r1.xyzw, r0.xyzw
mov oT2.z, -r0.xxxx
dp4 r1.w, v0.xyzw, c3.xyzw
dp4 oPos.x, r1.xyzw, c8.xyzw
dp4 oPos.y, r1.xyzw, c9.xyzw
dp4 oPos.z, r1.xyzw, c10.xyzw
dp4 oPos.w, r1.xyzw, c11.xyzw
add r0.x, r1.zzzz, c16.xxxx
mul r0.x, r0.xxxx, c16.yyyy
max r0.x, r0.xxxx, c7.yyyy
min oFog.xyzw, r0.xxxx, c7.xxxx
mul r0.xy, v1.xyzw, c15.xxxx
mov r0.z, c7.xxxx
dp3 oT0.x, r0.xyzw, c12.xyww
dp3 oT0.y, r0.xyzw, c13.xyww
mov oT0.z, c14.xxxx
mov oT1.x, v2.xxxx

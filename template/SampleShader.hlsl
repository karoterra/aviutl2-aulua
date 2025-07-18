Texture2D image : register(t0);

cbuffer constant0 : register(b0) {
    float channel;
}

float4 psmain(float4 pos : SV_Position) : SV_Target {
    int2 ipos = int2(pos.xy);
    float4 color = image[ipos];
    int ichannel = int(channel);

    if (ichannel == 0) {
        color.gb = 0.0;
    } else if (ichannel == 1) {
        color.rb = 0.0;
    } if (ichannel == 2) {
        color.rg = 0.0;
    }

    return color;
}

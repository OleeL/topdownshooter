local function zoom_decay(x)
    local alpha = 0.005 -- Decay rate
    return math.exp(-alpha * (x - 100))
end

local function drawCanvas(x, y, zoom, draw)
    love.graphics.push()
    local scale = zoom_decay(zoom)
    love.graphics.translate(
        (-x * scale) + (love.graphics.getWidth() * 0.5),
        (-y * scale) + (love.graphics.getHeight() * 0.5)
    )
    love.graphics.scale(scale, scale)
    draw()
    love.graphics.pop()
end

return drawCanvas

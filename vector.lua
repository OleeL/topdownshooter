local vector = {
    distance = function(entity1, entity2)
        return math.sqrt(((entity1.x - entity2.x) ^ 2) + ((entity1.y - entity2.y) ^ 2))
    end
}

return vector;

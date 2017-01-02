-- brute-force search for optimal trimming/"forgetting" strategies
import Debug.Trace
import Data.Foldable
import Data.Word
import Data.Bits

-- budget: maximum amount of memory allowed to be used
-- cost: number of recomputes

recompute :: Int -> Word64 -> Maybe (Int, Word64)
recompute budget cache = do
  let cost = countTrailingZeros cache
  let cache' = cache .|. (bit cost - 1)
  optTrim <- optimalTrim (budget - 1) (cache' `shiftR` 1)
  pure (cost, (optTrim `shiftL` 1) .|. 1)

reverseSweepCost :: Int -> Word64 -> Maybe Int
reverseSweepCost budget cache
  | cache == 0 = Just 0
  | cache .&. 1 /= 0 = reverseSweepCost budget (cache `shiftR` 1)
  | otherwise = do
      (cost, cache') <- recompute budget cache
      c2 <- reverseSweepCost budget cache'
      pure (cost + c2)

getBits :: Show b => FiniteBits b => b -> [Int]
getBits w =
  foldl' (\ l i -> if testBit w i then i : l else l)
  [] [0 .. finiteBitSize w - 1 - countLeadingZeros w]

optimalTrim :: Int -> Word64 -> Maybe Word64
optimalTrim budget cache
  | used <= budget = Just cache
  | otherwise = do
    let pt = weigh <$> possibleTrims (used - budget) cache
    case pt of
      [] -> Nothing
      _  -> Just (snd (minimum pt))
  where
    used = popCount cache
    weigh c = (reverseSweepCost budget c, c)

possibleTrims :: Int -> Word64 -> [Word64]
possibleTrims 0 c = pure c
possibleTrims n c = do
  i <- tail (getBits c) -- avoid unsetting the last bit
  possibleTrims (n - 1) (c `xor` bit i)

fromStr :: String -> Word64
fromStr s = go 0 s
  where go w "" = w
        go w (' ' : xs) = go (w `shiftL` 1) xs
        go w (_   : xs) = go ((w `shiftL` 1) + 1) xs

toStr :: Word64 -> String
toStr = reverse . go
  where go 0 = ""
        go w = (if testBit w 0 then 'x' else ' ') : go (w `shiftR` 1)

reverseSweep :: Int -> Word64 -> IO ()
reverseSweep budget cache
  | cache == 0 = pure ()
  | cache .&. 1 /= 0 = do
      print ("sweeping", toStr cache, 0::Int)
      reverseSweep budget (cache `shiftR` 1)
  | otherwise = do
      case recompute budget cache of
        Nothing -> do
          print "impossible"
        Just (cost, cache') -> do
          print ("sweeping", toStr cache, cost)
          reverseSweep budget cache'

main :: IO ()
main = do
  let b = 4 -- budget
  let initial = "x x x x"
  let t = case optimalTrim b (fromStr initial) of
            Just x -> x
            Nothing -> error "no solution"
  print ("result", toStr t)
  print ("cost", reverseSweepCost b t)
  reverseSweep b t
